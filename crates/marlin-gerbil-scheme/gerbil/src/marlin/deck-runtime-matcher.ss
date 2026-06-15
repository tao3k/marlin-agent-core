;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Higher-order matcher objects for Scheme-owned policy composition.

package: marlin

(import (only-in :clan/poo/object .get .o))

(export marlin-deck-runtime-high-order-matcher-kind
        make-marlin-deck-runtime-high-order-matcher
        marlin-deck-runtime-high-order-matcher-match?
        marlin-deck-runtime-and-matcher
        marlin-deck-runtime-or-matcher
        marlin-deck-runtime-not-matcher
        marlin-deck-runtime-command-prefix-matcher
        marlin-deck-runtime-agent-class-matcher
        marlin-deck-runtime-policy-name-matcher)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-high-order-matcher-kind
  "marlin-deck-runtime.high-order-matcher.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-high-order-matcher
      matcher-name-value
      matcher-procedure-value)
  (.o kind: marlin-deck-runtime-high-order-matcher-kind
      name: matcher-name-value
      predicate: matcher-procedure-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-high-order-matcher-match?
      matcher context policy command agent-scope)
  (if matcher
    ((.get matcher predicate) context policy command agent-scope)
    #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-and-matcher matcher-name matchers)
  (make-marlin-deck-runtime-high-order-matcher
   matcher-name
   (lambda (context policy command agent-scope)
     (andmap (lambda (matcher)
               (marlin-deck-runtime-high-order-matcher-match?
                matcher context policy command agent-scope))
             matchers))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-or-matcher matcher-name matchers)
  (make-marlin-deck-runtime-high-order-matcher
   matcher-name
   (lambda (context policy command agent-scope)
     (ormap (lambda (matcher)
              (marlin-deck-runtime-high-order-matcher-match?
               matcher context policy command agent-scope))
            matchers))))

;;; Boundary: Negated matchers model deny/defer carve-outs.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-not-matcher matcher-name matcher)
  (make-marlin-deck-runtime-high-order-matcher
   matcher-name
   (lambda (context policy command agent-scope)
     (not
      (marlin-deck-runtime-high-order-matcher-match?
       matcher
       context
       policy
       command
       agent-scope)))))

;;; Boundary: Command prefix matching stays composable in Scheme.
;; MarlinResult <- MarlinInput
(def (matcher-string-prefix? prefix value)
  (let (prefix-length (string-length prefix))
    (and (>= (string-length value) prefix-length)
         (string=? prefix (substring value 0 prefix-length)))))

;;; Boundary: Command prefix matcher covers common hook routing logic.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-command-prefix-matcher matcher-name prefixes)
  (make-marlin-deck-runtime-high-order-matcher
   matcher-name
   (lambda (_context _policy command _agent-scope)
     (ormap (lambda (prefix)
              (matcher-string-prefix? prefix command))
            prefixes))))

;;; Boundary: Agent class matcher routes custom/customer agents.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-agent-class-matcher matcher-name agent-classes)
  (make-marlin-deck-runtime-high-order-matcher
   matcher-name
   (lambda (context _policy _command _agent-scope)
     (if (member (.get context agent-class) agent-classes) #t #f))))

;;; Boundary: Policy name matcher routes model/policy templates by id.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-policy-name-matcher matcher-name policy-names)
  (make-marlin-deck-runtime-high-order-matcher
   matcher-name
   (lambda (_context policy _command _agent-scope)
     (if (member (.get policy name) policy-names) #t #f))))
