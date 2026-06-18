import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";

import { useAuth } from "./auth";
import LibraryBrowser from "./LibraryBrowser";
import ModalDialog from "./ModalDialog";
import SignInView from "./SignInView";
import { useFolderBrowser } from "./useFolderBrowser";
import "./library.css";

type View = "loading" | "signin" | "library";

/**
 * The scene-library modal. Resolves which view to show from auth state and renders it inside
 * the shared accessible `ModalDialog`; the sign-in form and the folder browser (with its
 * save/create/rename/move/delete concerns) live in their own components.
 */
export default function LibraryPanel({
  api,
  onClose,
}: {
  api: ExcalidrawImperativeAPI;
  onClose: () => void;
}) {
  const auth = useAuth();
  const browser = useFolderBrowser(auth.signedIn, auth.refresh);
  const view: View = auth.loading ? "loading" : auth.signedIn ? "library" : "signin";

  return (
    <ModalDialog title="Scene library" onClose={onClose}>
      {view === "loading" && <p className="lib-message">Loading…</p>}
      {view === "signin" && <SignInView auth={auth} />}
      {view === "library" && (
        <LibraryBrowser api={api} browser={browser} auth={auth} onClose={onClose} />
      )}
    </ModalDialog>
  );
}
