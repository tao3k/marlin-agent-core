// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::time::Duration;

use super::{
    app::{App, REPLY_QUEUE_BYTES, process_replies},
    event_queue::{ReplyError, reply_channel},
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelEndpointContractError, ModelGatewayCompletionChoice,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayTransport,
    ModelStreamInterruption, assistant_gateway_message,
};
use marlin_agent_stream::{ModelStreamChunk, ModelStreamEvent};

#[tokio::test]
async fn queue_accepts_normal_metadata_with_maximum_sized_text_completion() {
    let (sender, mut receiver) = reply_channel(4, REPLY_QUEUE_BYTES);
    let response = ModelGatewayCompletionResponse::new(
        "id",
        "model",
        vec![ModelGatewayCompletionChoice::new(
            0,
            assistant_gateway_message("a".repeat(16 * 1024 * 1024)),
            None,
        )],
    );
    assert!(
        sender
            .send(1, Ok(ModelStreamEvent::Completed { response }))
            .await
            .unwrap()
    );
    assert!(matches!(
        receiver.recv().await.unwrap().into_parts().1,
        Ok(ModelStreamEvent::Completed { .. })
    ));
}

#[tokio::test]
async fn byte_capacity_blocks_until_the_consumer_releases_the_event() {
    let (sender, mut receiver) = reply_channel(4, 128);
    assert!(
        sender
            .send(
                1,
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                    1,
                    "a".repeat(80)
                )))
            )
            .await
            .unwrap()
    );
    let sender2 = sender.clone();
    let task = tokio::spawn(async move {
        sender2
            .send(
                1,
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                    2,
                    "b".repeat(80),
                ))),
            )
            .await
    });
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    let first = receiver.recv().await.unwrap();
    tokio::task::yield_now().await;
    assert!(!task.is_finished(), "the event still owns its permit");
    drop(first);
    assert!(
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
    );
    let (_, second) = receiver.recv().await.unwrap().into_parts();
    assert!(matches!(second, Ok(ModelStreamEvent::Chunk(chunk)) if chunk.sequence == 2));
}

#[tokio::test]
async fn oversized_event_is_a_terminal_failure_not_silent_loss() {
    let (sender, mut receiver) = reply_channel(4, 128);
    let accepted = sender
        .send(
            7,
            Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                1,
                "a".repeat(256),
            ))),
        )
        .await
        .unwrap();
    assert!(!accepted);
    let (id, reply) = receiver.recv().await.unwrap().into_parts();
    assert_eq!(id, 7);
    assert!(matches!(
        reply,
        Err(ReplyError::QueueLimit {
            limit_bytes: 128,
            actual_bytes: 256
        })
    ));
}

#[tokio::test]
async fn typed_stream_uncertainty_survives_queue_admission() {
    let (sender, mut receiver) = reply_channel(1, 128);
    assert!(
        sender
            .send(
                3,
                Err(ModelGatewayError::StreamInterrupted(
                    ModelStreamInterruption::InvalidSequence {
                        expected: 4,
                        actual: 9
                    }
                ))
            )
            .await
            .unwrap()
    );
    assert!(matches!(
        receiver.recv().await.unwrap().into_parts(),
        (
            3,
            Err(ReplyError::Gateway(ModelGatewayError::StreamInterrupted(
                ModelStreamInterruption::InvalidSequence {
                    expected: 4,
                    actual: 9
                }
            )))
        )
    ));
}

#[tokio::test]
async fn oversized_gateway_error_is_rejected_using_its_owned_capacity() {
    let (sender, mut receiver) = reply_channel(1, 128);
    let mut message = String::with_capacity(256);
    message.push_str("small text");
    assert!(
        !sender
            .send(5, Err(ModelGatewayError::Completion(message)))
            .await
            .unwrap()
    );
    assert!(matches!(
        receiver.recv().await.unwrap().into_parts(),
        (
            5,
            Err(ReplyError::QueueLimit {
                limit_bytes: 128,
                actual_bytes: 256
            })
        )
    ));
}

#[tokio::test]
async fn endpoint_error_identifier_capacity_is_accounted() {
    let (sender, mut receiver) = reply_channel(1, 128);
    let mut provider = String::with_capacity(256);
    provider.push_str("invalid/provider");
    let error = ModelGatewayError::EndpointContract(
        ModelEndpointContractError::ProviderContainsModelSeparator {
            provider: provider.into(),
        },
    );
    assert!(!sender.send(6, Err(error)).await.unwrap());
    assert!(matches!(
        receiver.recv().await.unwrap().into_parts(),
        (
            6,
            Err(ReplyError::QueueLimit {
                limit_bytes: 128,
                actual_bytes: 256
            })
        )
    ));
}

#[tokio::test]
async fn cancelling_a_blocked_sender_releases_its_byte_reservation() {
    let (sender, mut receiver) = reply_channel(1, 256);
    sender
        .send(
            1,
            Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                1,
                "a".repeat(80),
            ))),
        )
        .await
        .unwrap();
    let other = sender.clone();
    let blocked = tokio::spawn(async move {
        other
            .send(
                1,
                Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                    2,
                    "b".repeat(80),
                ))),
            )
            .await
    });
    tokio::task::yield_now().await;
    assert!(!blocked.is_finished());
    blocked.abort();
    let _ = blocked.await;
    drop(receiver.recv().await.unwrap());
    let sent = tokio::time::timeout(
        Duration::from_secs(2),
        sender.send(
            1,
            Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                3,
                "c".repeat(200),
            ))),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(sent);
}

#[tokio::test]
async fn completion_waits_for_capacity_and_remains_ordered() {
    let (sender, mut receiver) = reply_channel(2, 256);
    sender
        .send(
            1,
            Ok(ModelStreamEvent::Chunk(ModelStreamChunk::new(
                1,
                "a".repeat(200),
            ))),
        )
        .await
        .unwrap();
    let other = sender.clone();
    let task = tokio::spawn(async move {
        let response = ModelGatewayCompletionResponse::new(
            "receipt",
            "gpt-test",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message("done"),
                Some("stop".into()),
            )],
        );
        other
            .send(1, Ok(ModelStreamEvent::Completed { response }))
            .await
    });
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    let first = receiver.recv().await.unwrap();
    assert!(matches!(&first.reply, Ok(ModelStreamEvent::Chunk(_))));
    drop(first);
    assert!(
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
    );
    let (_, final_event) = receiver.recv().await.unwrap().into_parts();
    assert!(
        matches!(final_event, Ok(ModelStreamEvent::Completed { response }) if response.id == "receipt")
    );
}

#[test]
fn event_batches_leave_a_control_input_opportunity() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut receiver) = reply_channel(32, 128);
    runtime.block_on(async {
        for _ in 0..20 {
            sender
                .send(
                    1,
                    Ok(ModelStreamEvent::Started {
                        transport: ModelGatewayTransport::Sse,
                    }),
                )
                .await
                .unwrap();
        }
    });
    let mut app = App::new(ModelEndpoint::new("openai", "gpt-test"));
    assert_eq!(process_replies(&mut app, &mut receiver, 16), 16);
    assert_eq!(receiver.len(), 4);
    assert_eq!(process_replies(&mut app, &mut receiver, 16), 4);
}
