// Project model entry. Load this file together with dev/sysml.
// Model map and view commands: dev/sysml/README.org
package MarlinAgentCORE {
    private import Views::*;
    private import MarlinPluginEngine::*;
    private import MarlinPluginLifecycle::*;
    private import MarlinPluginExecution::*;

    part <'MAR-PROJECT'> project {
        doc /* Marlin Agent CORE. This entry selects the plugin-engine
               architecture and lifecycle viewpoint. */
    }

    dependency currentRuntime from project to actual::runtime;
    dependency targetEngine from project to target::pluginEngine;
    dependency activationLifecycle from project to ActivationLifecycle;
    dependency invocationLifecycle from project to InvocationLifecycle;
    dependency requestExecutionPath from project to PluginRequestPath;
    dependency performanceRequirement from project to requirements::highPerformanceExecution;

    view projectStructure {
        expose project;
        expose MarlinPluginEngine::actual;
        expose MarlinPluginEngine::target;
        expose requirements::highPerformanceExecution;
        render asTreeDiagram;
    }

    view pluginTransport {
        expose target::pluginEngine;
        expose target::clientShell;
        expose target::tsPanel;
        expose target::schemeVm;
        expose target::scopedPanelChannel;
        expose target::grpcChannel;
        expose target::schemeCAbiChannel;
        render asInterconnectionDiagram;
    }

    view performanceDesign {
        expose requirements::highPerformanceExecution;
        expose target::pluginEngine::runtime;
        expose target::pluginEngine::schemeCAbiBridge;
        expose target::pluginEngine::grpcService;
        render asTreeDiagram;
    }

    view pluginRequestPath : StandardViewDefinitions::ActionFlowView {
        expose PluginRequestPath;
    }
}
