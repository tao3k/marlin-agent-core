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
    dependency singleVmExecution from project to MarlinSingleVmRuntime::singleVmRuntime;
    dependency tuiDesign from project to MarlinTuiArchitecture::target::terminalClient;
    dependency contextOrgDesign from project to MarlinTuiArchitecture::target::contextOrgAuthority;
    dependency contextOrgLifecycle from project to MarlinContextOrgLifecycle::OrgTurnLifecycle;
    dependency tuiTurnLifecycle from project to MarlinTuiLifecycle::TurnLifecycle;

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
        expose MarlinSingleVmRuntime::singleVmRuntime;
        expose MarlinSingleVmRuntime::singleVmOwnership;
        expose MarlinSingleVmRuntime::boundedReadyBatching;
        expose MarlinSingleVmRuntime::terminalAccounting;
        expose MarlinSingleVmRuntime::measuredExecution;
        expose requirements::highPerformanceExecution;
        expose target::pluginEngine::runtime;
        expose target::pluginEngine::schemeCAbiBridge;
        expose target::pluginEngine::grpcService;
        render asTreeDiagram;
    }

    view pluginRequestPath : StandardViewDefinitions::ActionFlowView {
        expose PluginRequestPath;
    }

    view tuiPerformanceDesign {
        expose MarlinTuiArchitecture::actual;
        expose MarlinTuiArchitecture::target;
        expose MarlinTuiArchitecture::requirements;
        render asTreeDiagram;
    }
}
