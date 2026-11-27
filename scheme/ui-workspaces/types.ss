;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contracts for Marlin's Doom-inspired Workspace (persp-mode), recursive
;;; multi-window manager (winum + winner-mode + dedicated windows), and Popup
;;; Rule (set-popup-rule!) manager.
(import (only-in :clan/poo/object .cc object?)
        (only-in :clan/poo/type Bool Exactly Fun List Object Record String Symbol Type.))
(export PopupSideContract PopupQuitContract PopupTtlContract SplitDirectionContract
        UiWorkspacesScopeContract)

(def IntContract
  (.cc Type. '.element? exact-integer?))

(def PopupSideContract
  (.cc Type. '.element? (lambda (x) (and (memq x '(bottom right left top)) #t))))

(def PopupQuitContract
  (.cc Type. '.element? (lambda (x) (and (memq x '(current other never)) #t))))

(def PopupTtlContract
  (.cc Type. '.element? (lambda (x) (and (memq x '(ephemeral session persistent)) #t))))

(def SplitDirectionContract
  (.cc Type. '.element? (lambda (x) (and (memq x '(vertical horizontal)) #t))))

(def RatioContract
  (.cc Type. '.element? number?))

(def MaybeObjectContract
  (.cc Type. '.element? (lambda (x) (or (not x) (object? x)))))

(def VectorContract
  (.cc Type. '.element? vector?))

(def UiWorkspacesScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-workspaces)]
               default-popup-rules: [(List Object)]
               make-state: [(Fun Object)]
               make-popup-rule: [(Fun Object <- String Symbol RatioContract Bool Symbol Symbol Bool)]
               add-popup-rule: [(Fun Object <- Object Object)]
               match-popup-rule: [(Fun MaybeObjectContract <- Object String)]
               create-workspace: [(Fun Object <- Object String String String)]
               switch-workspace: [(Fun Object <- Object String)]
               kill-workspace: [(Fun Object <- Object String)]
               open-buffer: [(Fun Object <- Object String)]
               dismiss-popup: [(Fun Object <- Object)]
               toggle-popup: [(Fun Object <- Object)]
               raise-popup: [(Fun Object <- Object)]
               workspace-buffers: [(Fun (List String) <- Object)]
               all-workspace-buffers: [(Fun (List String) <- Object)]
               isolate-buffer-candidates: [(Fun (List String) <- Object (List String) Bool)]
               window-list: [(Fun (List Object) <- Object)]
               window-count: [(Fun IntContract <- Object)]
               focused-window: [(Fun Object <- Object)]
               window-number: [(Fun IntContract <- Object String)]
               focused-window-number: [(Fun IntContract <- Object)]
               split-window: [(Fun Object <- Object SplitDirectionContract String)]
               select-window-number: [(Fun Object <- Object IntContract)]
               other-window: [(Fun Object <- Object IntContract)]
               delete-window: [(Fun Object <- Object)]
               delete-other-windows: [(Fun Object <- Object)]
               swap-windows: [(Fun Object <- Object)]
               resize-window: [(Fun Object <- Object RatioContract)]
               balance-windows: [(Fun Object <- Object)]
               toggle-dedicated: [(Fun Object <- Object)]
               winner-undo: [(Fun Object <- Object)]
               winner-redo: [(Fun Object <- Object)]
               modeline-window-indicator: [(Fun String <- Object String Bool)]
               project-window-tree: [(Fun VectorContract <- Object)]
               project-state: [(Fun VectorContract <- Object)]
               project-popup-rules: [(Fun VectorContract <- Object)])
       identity: 'marlin/ui-workspaces))
