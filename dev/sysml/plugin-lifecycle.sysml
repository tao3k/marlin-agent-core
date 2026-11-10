// Target lifecycle. Rationale: ../../docs/40-rfcs/40.166-plugin-contract.org::#plugin-lifecycle
package MarlinPluginLifecycle {
    private import MarlinPluginEngine::*;

    doc /* Viewpoint: Rust-owned plugin activation and run lifecycle.
           Transition events represent observations from their owners. */

    attribute def BundleVerified;
    attribute def ActivationAdmitted;
    attribute def ActivationDenied;
    attribute def AdmissionClosed;
    attribute def GenerationQuiesced;

    state def <'MAR-PLG-L01'> ActivationLifecycle {
        doc /* One activation generation. A successful later enable creates
               a fresh generation. Quiescence accounts for admitted effects. */
        first start then uninstalled;
        state uninstalled;
        state installed;
        state active;
        state draining;
        state closed;
        transition install first uninstalled accept BundleVerified then installed;
        transition enable first installed accept ActivationAdmitted then active;
        transition denyEnable first installed accept ActivationDenied then installed;
        transition disable first active accept AdmissionClosed then draining;
        transition finishDisable first draining accept GenerationQuiesced then closed;
    }

    attribute def PlanRequired;
    attribute def PlanAdmitted;
    attribute def AdmissionDenied;
    attribute def RunCompleted;
    attribute def RunFailed;
    attribute def RunCancelled;

    state def <'MAR-PLG-L02'> InvocationLifecycle {
        doc /* Begins at Rust run admission. A plan may require a Scheme
               projection. Rust rechecks live authority at each effect. */
        first start then admitted;
        state admitted;
        state planning;
        state executing;
        state completed;
        state denied;
        state failed;
        state cancelled;
        transition requestPlan first admitted accept PlanRequired then planning;
        transition executeReadyPlan first admitted accept PlanAdmitted then executing;
        transition executeNewPlan first planning accept PlanAdmitted then executing;
        transition denyReadyPlan first admitted accept AdmissionDenied then denied;
        transition denyNewPlan first planning accept AdmissionDenied then denied;
        transition denyEffect first executing accept AdmissionDenied then denied;
        transition complete first executing accept RunCompleted then completed;
        transition failPlan first planning accept RunFailed then failed;
        transition failExecution first executing accept RunFailed then failed;
        transition cancelAdmission first admitted accept RunCancelled then cancelled;
        transition cancelPlan first planning accept RunCancelled then cancelled;
        transition cancelExecution first executing accept RunCancelled then cancelled;
    }

    dependency activationOwnedByGate from ActivationLifecycle to target::pluginEngine::activationGate;
    dependency invocationOwnedByRuntime from InvocationLifecycle to target::pluginEngine::runtime;
    dependency invocationRecordsEvidence from InvocationLifecycle to target::pluginEngine::evidence;
}
