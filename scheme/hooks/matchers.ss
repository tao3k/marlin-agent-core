;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Higher-order matcher objects for Scheme-owned policy composition.

(import (only-in :clan/poo/object .get .o))

(export hook-matcher-kind
        make-hook-matcher
        hook-matcher-match?
        hook-and-matcher
        hook-or-matcher
        hook-not-matcher
        hook-command-prefix-matcher
        hook-agent-class-matcher
        hook-policy-name-matcher)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def hook-matcher-kind
  "marlin-deck-runtime.high-order-matcher.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-hook-matcher
      matcher-name-value
      matcher-procedure-value)
  (.o kind: hook-matcher-kind
      name: matcher-name-value
      predicate: matcher-procedure-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-matcher-match?
      matcher context policy command agent-scope)
  (if matcher
    ((.get matcher predicate) context policy command agent-scope)
    #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-and-matcher matcher-name matchers)
  (make-hook-matcher
   matcher-name
   (lambda (context policy command agent-scope)
     (andmap (cut hook-matcher-match? <> context policy command agent-scope)
             matchers))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-or-matcher matcher-name matchers)
  (make-hook-matcher
   matcher-name
   (lambda (context policy command agent-scope)
     (ormap (cut hook-matcher-match? <> context policy command agent-scope)
            matchers))))

;;; Boundary: Negated matchers model deny/defer carve-outs.
;; MarlinResult <- MarlinInput
(def (hook-not-matcher matcher-name matcher)
  (make-hook-matcher
   matcher-name
   (lambda (context policy command agent-scope)
     (not
      (hook-matcher-match?
       matcher
       context
       policy
       command
       agent-scope)))))

;;; Boundary: Command prefix matcher covers common hook routing logic.
;; MarlinResult <- MarlinInput
(def (hook-command-prefix-matcher matcher-name prefixes)
  (make-hook-matcher
   matcher-name
   (lambda (_context _policy command _agent-scope)
     (ormap (cut string-prefix? <> command) prefixes))))

;;; Boundary: Agent class matcher routes custom/customer agents.
;; MarlinResult <- MarlinInput
(def (hook-agent-class-matcher matcher-name agent-classes)
  (make-hook-matcher
   matcher-name
   (lambda (context _policy _command _agent-scope)
     (if (member (.get context agent-class) agent-classes) #t #f))))

;;; Boundary: Policy name matcher routes model/policy templates by id.
;; MarlinResult <- MarlinInput
(def (hook-policy-name-matcher matcher-name policy-names)
  (make-hook-matcher
   matcher-name
   (lambda (_context policy _command _agent-scope)
     (if (member (.get policy name) policy-names) #t #f))))
