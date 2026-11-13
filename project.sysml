// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Project model entry. Load this file together with dev/sysml.
// Model map and view commands: dev/sysml/README.org
package MarlinAgentCORE {
    private import Views::*;
    private import MarlinPluginEngine::*;
    private import MarlinPluginLifecycle::*;
    private import MarlinPluginExecution::*;

    part <'MAR-PROJECT'> project {
        doc /* Marlin AI Agent Platform. This repository focuses on Agent CORE,
               its runtime and extension contracts, and application transport.
               External Web, desktop and mobile applications live in separate
               repositories and consume the App Server schema. */
    }

    dependency currentRuntime from project to actual::runtime;
    dependency targetEngine from project to target::pluginEngine;
    dependency activationLifecycle from project to ActivationLifecycle;
    dependency invocationLifecycle from project to InvocationLifecycle;
    dependency requestExecutionPath from project to PluginRequestPath;
    dependency performanceRequirement from project to requirements::highPerformanceExecution;
    dependency singleVmExecution from project to MarlinSingleVmRuntime::singleVmRuntime;
    dependency processLifecycle from project to MarlinProcessLifecycle::managedProcess;
    dependency processQuotaRequirement from project to MarlinProcessLifecycle::quotaHeldUntilExit;
    dependency telemetryExport from project to MarlinTelemetry::telemetry;
    dependency rustPackageOwnership from project to MarlinRustPackageOwnership::packages;
    dependency eventBusContract from project to MarlinEventBusContract::eventBusDesign;
    dependency externalAppServer from project to MarlinAppServer::appServer;
    dependency tuiDesign from project to MarlinTuiArchitecture::target::terminalClient;
    dependency declarativeTuiEngine from project to MarlinDeclarativeUi::engine;
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

    view appServerTransport {
        expose MarlinAppServer::externalClient;
        expose MarlinAppServer::weaverInterop;
        expose MarlinAppServer::appServer;
        expose MarlinAppServer::grpcChannel;
        expose MarlinAppServer::weaverGrpcChannel;
        expose MarlinAppServer::clientBuildIsolation;
        render asInterconnectionDiagram;
    }

    view appServerRecovery {
        expose MarlinAppServer::recoverSubmission;
        expose MarlinAppServer::boundedSessionRecovery;
        expose MarlinAppServer::recoverCancellationEvidence;
        expose MarlinAppServer::appServer;
        render asInterconnectionDiagram;
    }

    view applicationMessageFlow : StandardViewDefinitions::ActionFlowView {
        expose MarlinAppServer::ApplicationMessageFlow;
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
        expose MarlinDeclarativeUi::engine;
        render asTreeDiagram;
    }

    view rustPackageBoundaries {
        expose MarlinRustPackageOwnership::packages;
        expose MarlinRustPackageOwnership::scheme;
        render asInterconnectionDiagram;
    }
}
