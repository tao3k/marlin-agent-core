use std::{
    future::{Future, poll_fn},
    sync::Arc,
    task::Poll,
};

use marlin_agent_protocol::{ModelRouteAdmissionRequest, ModelRouteRequest};
use marlin_agent_runtime::CompiledModelRouteResolver;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{DeckRouteAdmission, DeckRouteAdmissionRejection};

#[tokio::test]
async fn saturated_queue_rejects_and_cancelled_request_skips_vm_entry() {
    let (queue, mut jobs) = mpsc::channel(1);
    let service = DeckRouteAdmission {
        queue,
        resolver: Arc::new(CompiledModelRouteResolver::new(Vec::new()).unwrap()),
    };
    let request = ModelRouteAdmissionRequest::chat(ModelRouteRequest::command(["marlin", "chat"]));
    let cancellation = CancellationToken::new();
    let mut first = std::pin::pin!(service.admit(request.clone(), cancellation.clone()));
    poll_fn(|cx| {
        assert!(first.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(matches!(
        service.admit(request, CancellationToken::new()).await,
        Err(DeckRouteAdmissionRejection::Backpressure)
    ));
    cancellation.cancel();
    assert!(matches!(
        first.await,
        Err(DeckRouteAdmissionRejection::Cancelled)
    ));
    let queued = jobs.try_recv().expect("first admission enqueued");
    assert!(matches!(
        queued.check_active(),
        Err(DeckRouteAdmissionRejection::Cancelled)
    ));
}
