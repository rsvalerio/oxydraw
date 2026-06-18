import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Excalidraw, MainMenu } from "@excalidraw/excalidraw";
import type {
  ExcalidrawImperativeAPI,
  ExcalidrawInitialDataState,
} from "@excalidraw/excalidraw/types";

import { loadFromLocalStorage, saveToLocalStorage } from "./localData";
import {
  exportToShareLink,
  importFromShareLink,
  parseShareFragment,
} from "./share";
import { Collab } from "./collab/Collab";
import { parseRoomFragment } from "./collab/protocol";
import LibraryPanel from "./library/LibraryPanel";
import { oauthStartUrl } from "./library/api";
import { PROVIDER_LABELS, useAuth } from "./library/auth";

/**
 * OAuth sign-in failures redirect the browser back to `/?ext_auth_error=<message>` (the
 * server can't show UI mid-redirect). Read and strip the param so we can toast it once.
 */
function takeAuthError(): string | null {
  const params = new URLSearchParams(window.location.search);
  const message = params.get("ext_auth_error");
  if (message === null) {
    return null;
  }
  params.delete("ext_auth_error");
  const query = params.toString();
  window.history.replaceState(
    null,
    "",
    `${window.location.pathname}${query ? `?${query}` : ""}${window.location.hash}`,
  );
  return message;
}

/**
 * Resolve the scene to open with. A collaboration room (`#room=`) starts empty — the room's
 * peers / snapshot populate it, and loading local data first would leak it into the room. A
 * share link (`#json=`) loads the shared scene; otherwise we restore the last local scene.
 */
async function loadInitialScene(): Promise<ExcalidrawInitialDataState | null> {
  if (parseRoomFragment()) {
    return null;
  }
  const fragment = parseShareFragment();
  if (fragment) {
    try {
      const scene = await importFromShareLink(fragment.id, fragment.key);
      return {
        elements: scene.elements,
        appState: scene.appState,
        files: scene.files,
      };
    } catch (error) {
      console.error("failed to open shared scene", error);
    }
  }
  const local = loadFromLocalStorage();
  return local
    ? { elements: local.elements, appState: local.appState, files: local.files }
    : null;
}

export default function App() {
  const [api, setApi] = useState<ExcalidrawImperativeAPI | null>(null);
  const collabRef = useRef<Collab | null>(null);
  const [collaborating, setCollaborating] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [libraryOpen, setLibraryOpen] = useState(false);
  const initialData = useMemo(() => loadInitialScene(), []);
  const auth = useAuth();

  // Surface an OAuth redirect error (e.g. account not allowed) once the editor is ready.
  useEffect(() => {
    if (!api) {
      return;
    }
    const error = takeAuthError();
    if (error) {
      api.setToast({ message: error, duration: 4000, closable: true });
    }
  }, [api]);

  // Build the collab manager once the editor API is ready, and auto-join a room URL.
  useEffect(() => {
    if (!api) {
      return;
    }
    const collab = new Collab(api, {
      onStarted: (url) => {
        setCollaborating(true);
        void navigator.clipboard?.writeText(url);
        api.setToast({ message: "Collaboration link copied to clipboard", duration: 3000 });
      },
      onStopped: () => setCollaborating(false),
    });
    collabRef.current = collab;

    const room = parseRoomFragment();
    if (room) {
      void collab.joinRoom(room.roomId, room.key);
    }
    return () => collab.stop();
  }, [api]);

  const handleShare = useCallback(async () => {
    if (!api || sharing) {
      return;
    }
    setSharing(true);
    try {
      const url = await exportToShareLink(
        api.getSceneElements(),
        api.getAppState(),
        api.getFiles(),
      );
      window.history.replaceState(null, "", url);
      await navigator.clipboard?.writeText(url);
      api.setToast({ message: "Share link copied to clipboard", duration: 3000 });
    } catch (error) {
      console.error("failed to create share link", error);
      api.setToast({ message: "Could not create share link", duration: 3000 });
    } finally {
      setSharing(false);
    }
  }, [api, sharing]);

  const handleSignOut = useCallback(async () => {
    await auth.signOut();
    api?.setToast({ message: "Signed out", duration: 2000 });
  }, [auth, api]);

  const handleCollabToggle = useCallback(() => {
    const collab = collabRef.current;
    if (!collab) {
      return;
    }
    if (collab.isActive) {
      collab.stop();
    } else {
      void collab.startNewRoom();
    }
  }, []);

  return (
    <div className="oxydraw-app">
      <Excalidraw
        excalidrawAPI={setApi}
        initialData={initialData}
        isCollaborating={collaborating}
        onChange={(elements, appState, files) => {
          collabRef.current?.onLocalChange(elements);
          if (!collaborating) {
            saveToLocalStorage(elements, appState, files);
          }
        }}
        onPointerUpdate={(payload) =>
          collabRef.current?.onPointerUpdate(payload.pointer, payload.button)
        }
      >
        <MainMenu>
          <MainMenu.Item onSelect={handleShare}>Share link</MainMenu.Item>
          <MainMenu.Item onSelect={handleCollabToggle}>
            {collaborating ? "Stop collaboration" : "Start collaboration"}
          </MainMenu.Item>
          <MainMenu.Separator />
          <MainMenu.DefaultItems.LoadScene />
          <MainMenu.DefaultItems.SaveToActiveFile />
          <MainMenu.DefaultItems.Export />
          <MainMenu.DefaultItems.SaveAsImage />
          <MainMenu.DefaultItems.ClearCanvas />
          <MainMenu.Separator />
          <MainMenu.DefaultItems.ToggleTheme />
          <MainMenu.DefaultItems.ChangeCanvasBackground />
          {auth.authEnabled && (
            <>
              <MainMenu.Separator />
              <MainMenu.Group title="Workspace">
                {auth.signedIn ? (
                  <>
                    <MainMenu.ItemCustom className="oxydraw-account-name">
                      {auth.me?.user.name || auth.me?.user.email || "Signed in"}
                    </MainMenu.ItemCustom>
                    <MainMenu.Item onSelect={() => setLibraryOpen(true)}>
                      Scene library
                    </MainMenu.Item>
                    <MainMenu.Item onSelect={handleSignOut}>Sign out</MainMenu.Item>
                  </>
                ) : (
                  <>
                    {auth.providers.providers.map((provider) => (
                      <MainMenu.Item
                        key={provider}
                        onSelect={() => {
                          // Same-tab top-level navigation: the OAuth flow redirects back
                          // to "/". ItemLink can't be used — it hardcodes target="_blank".
                          window.location.href = oauthStartUrl(provider);
                        }}
                      >
                        Sign in with {PROVIDER_LABELS[provider] ?? provider}
                      </MainMenu.Item>
                    ))}
                    {auth.providers.password && (
                      <MainMenu.Item onSelect={() => setLibraryOpen(true)}>
                        Sign in with password…
                      </MainMenu.Item>
                    )}
                  </>
                )}
              </MainMenu.Group>
            </>
          )}
        </MainMenu>
      </Excalidraw>
      {libraryOpen && api && (
        <LibraryPanel api={api} onClose={() => setLibraryOpen(false)} />
      )}
    </div>
  );
}
