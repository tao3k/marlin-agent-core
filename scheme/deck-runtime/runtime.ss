;;; Gerbil schedules Context admissions as green threads on demand.

(import (only-in :clan/poo/object .get .o))

(export open-deck-runtime)

(def (open-deck-runtime admit-context)
  (def owner (current-thread))
  (def closed? #f)
  (def (admit-turn request)
    (def admitted (admit-context request))
    ;; Complete the lazy POO identity inside the handling green thread.
    (unless (string? (.get admitted semantic-digest))
      (error "Context admission did not produce a Scope identity"))
    admitted)
  (def (assert-open)
    (unless (eq? owner (current-thread))
      (error "Deck runtime belongs to its opening Scheme thread"))
    (when closed? (error "Deck runtime is closed")))
  (def (admit-rows rows)
    (assert-open)
    (def count (vector-length rows))
    (def actors (make-vector count #f))
    (def results (make-vector count #f))
    ;; Join every started actor even when starting a later actor fails.
    (dynamic-wind void
      (lambda ()
        (let loop ((index 0))
          (when (< index count)
            (let (row (vector-ref rows index))
              (vector-set! actors index
                (spawn (lambda () (with-catch (lambda (_) #f) (lambda () (admit-turn row)))))))
            (loop (1+ index)))))
      (lambda ()
        (let loop ((index 0))
          (when (< index count)
            (let (actor (vector-ref actors index))
              (when actor
                (vector-set! results index
                  (with-catch (lambda (_) #f) (lambda () (thread-join! actor))))))
            (loop (1+ index))))))
    (vector-for-each
      (lambda (result) (unless result (error "Deck Context actor admission failed")))
      results)
    results)
  (.o evaluate-batch: admit-rows
      close: (lambda ()
               (assert-open)
               (set! closed? #t)
               #t)))
