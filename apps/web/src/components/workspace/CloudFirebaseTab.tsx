// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import {
  MdAssistChip,
  MdCircularProgress,
  MdElevatedCard,
  MdFilledButton,
  MdFilledTonalButton,
  MdIcon,
  MdOutlinedButton,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { auth, db, storage } from "../../firebase.ts";
import { signInWithPopup, GoogleAuthProvider, signOut, onAuthStateChanged, User } from "firebase/auth";
import { collection, addDoc, onSnapshot, query, limit, doc, deleteDoc } from "firebase/firestore";
import { ref, uploadBytesResumable, getDownloadURL, listAll } from "firebase/storage";
import { session } from "../../store.ts";

export function CloudFirebaseTab() {
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [authBusy, setAuthBusy] = useState(false);
  const [authError, setAuthError] = useState("");

  const [syncDocs, setSyncDocs] = useState<{ id: string; val: string }[]>();
  const [newDocText, setNewDocText] = useState("");
  const [syncBusy, setSyncBusy] = useState(false);

  const [files, setFiles] = useState<{ name: string; url: string; size?: number }[]>([]);
  const [uploadProgress, setUploadProgress] = useState<number | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  // 1. Auth Listener
  useEffect(() => {
    return onAuthStateChanged(auth, (user) => {
      setCurrentUser(user);
    });
  }, []);

  const loginWithGoogle = async () => {
    setAuthBusy(true);
    setAuthError("");
    try {
      const provider = new GoogleAuthProvider();
      const result = await signInWithPopup(auth, provider);
      const token = await result.user.getIdToken();
      session.signIn({
        id: result.user.uid,
        name: result.user.displayName || result.user.email || "Utilisateur Cloud",
        email: result.user.email || "",
        profile_image_url: result.user.photoURL || "/favicon.png",
        role: "admin",
        token: token,
      });
    } catch (err: any) {
      console.error(err);
      setAuthError(err.message || "Erreur d'authentification Google.");
    } finally {
      setAuthBusy(false);
    }
  };

  const handleLogout = async () => {
    setAuthBusy(true);
    try {
      await signOut(auth);
      session.signOut();
    } catch (err) {
      console.error(err);
    } finally {
      setAuthBusy(false);
    }
  };

  // 2. Firestore Sync Listener
  useEffect(() => {
    const q = query(collection(db, "sync_tests"), limit(10));
    const unsubscribe = onSnapshot(q, (snapshot) => {
      const docs = snapshot.docs.map((d) => ({
        id: d.id,
        val: d.data().text as string,
      }));
      setSyncDocs(docs);
    });
    return unsubscribe;
  }, []);

  const handleAddSyncDoc = async () => {
    if (!newDocText.trim()) return;
    setSyncBusy(true);
    try {
      await addDoc(collection(db, "sync_tests"), {
        text: newDocText,
        timestamp: Date.now(),
        author: currentUser?.email || "anonymous",
      });
      setNewDocText("");
    } catch (err) {
      console.error(err);
    } finally {
      setSyncBusy(false);
    }
  };

  const handleDeleteSyncDoc = async (id: string) => {
    try {
      await deleteDoc(doc(db, "sync_tests", id));
    } catch (err) {
      console.error(err);
    }
  };

  // 3. Storage File Management
  const fetchStorageFiles = async () => {
    try {
      const storageRef = ref(storage, "uploads");
      const res = await listAll(storageRef);
      const fileList = await Promise.all(
        res.items.map(async (item) => {
          const url = await getDownloadURL(item);
          return { name: item.name, url };
        }),
      );
      setFiles(fileList);
    } catch (err) {
      console.error("Failed to fetch storage files:", err);
    }
  };

  useEffect(() => {
    fetchStorageFiles();
  }, []);

  const handleUploadFile = () => {
    if (!selectedFile) return;
    const fileRef = ref(storage, `uploads/${selectedFile.name}`);
    const uploadTask = uploadBytesResumable(fileRef, selectedFile);

    uploadTask.on(
      "state_changed",
      (snapshot) => {
        const progress = (snapshot.bytesTransferred / snapshot.totalBytes) * 100;
        setUploadProgress(progress);
      },
      (error) => {
        console.error("Upload error:", error);
        setUploadProgress(null);
      },
      () => {
        setUploadProgress(null);
        setSelectedFile(null);
        fetchStorageFiles();
      },
    );
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24, marginTop: 16 }}>
      {/* Overview Cards */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 16 }}>
        <MdElevatedCard style={{ padding: 20 }}>
          <div className="owui-row" style={{ gap: 12 }}>
            <MdIcon style={{ fontSize: 36, color: "#EA4335" }}>cloud</MdIcon>
            <div>
              <h3 style={{ margin: 0 }}>Google Cloud Platform</h3>
              <p className="owui-muted" style={{ margin: "4px 0 0", fontSize: 13 }}>
                Project: <strong>aphrody</strong> | Location: <strong>us-central1</strong>
              </p>
            </div>
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 16 }}>
            <MdAssistChip label="Gemini SDK Active" />
            <MdAssistChip label="Vertex AI API Active" />
            <MdAssistChip label="RAG Engine Ready" />
          </div>
        </MdElevatedCard>

        <MdElevatedCard style={{ padding: 20 }}>
          <div className="owui-row" style={{ gap: 12 }}>
            <MdIcon style={{ fontSize: 36, color: "#FFCA28" }}>local_fire_department</MdIcon>
            <div>
              <h3 style={{ margin: 0 }}>Firebase Integration</h3>
              <p className="owui-muted" style={{ margin: "4px 0 0", fontSize: 13 }}>
                Status: <strong>Connecté en temps réel</strong>
              </p>
            </div>
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 16 }}>
            <MdAssistChip label="Auth Module" />
            <MdAssistChip label="Cloud Firestore" />
            <MdAssistChip label="Cloud Storage" />
          </div>
        </MdElevatedCard>
      </div>

      {/* Authentication */}
      <MdOutlinedCard style={{ padding: 20 }}>
        <h3 style={{ margin: "0 0 16px" }} className="owui-row">
          <MdIcon style={{ marginRight: 8, color: "var(--md-sys-color-primary)" }}>fingerprint</MdIcon>
          Authentification Globale Firebase Auth
        </h3>
        
        {currentUser ? (
          <div className="owui-spread" style={{ alignItems: "center" }}>
            <div className="owui-row" style={{ gap: 16 }}>
              <img
                src={currentUser.photoURL || "/favicon.png"}
                alt="Avatar"
                style={{ width: 48, height: 48, borderRadius: "50%", border: "2px solid var(--md-sys-color-primary)" }}
              />
              <div>
                <strong style={{ fontSize: 16 }}>{currentUser.displayName || "Utilisateur"}</strong>
                <div style={{ fontSize: 13, color: "var(--md-sys-color-on-surface-variant)" }}>{currentUser.email}</div>
                <div style={{ fontSize: 11, color: "var(--md-sys-color-primary)", marginTop: 2 }}>
                  UID: {currentUser.uid}
                </div>
              </div>
            </div>
            <MdOutlinedButton onClick={handleLogout} disabled={authBusy}>
              Déconnexion
            </MdOutlinedButton>
          </div>
        ) : (
          <div>
            <p className="owui-muted" style={{ marginTop: 0 }}>
              Connectez-vous via Firebase Authentication pour bénéficier d'une synchronisation multi-appareils automatique de toutes vos conversations.
            </p>
            {authError && <p style={{ color: "var(--md-sys-color-error)" }}>{authError}</p>}
            <MdFilledButton onClick={loginWithGoogle} disabled={authBusy}>
              <MdIcon slot="icon">account_circle</MdIcon>
              Se connecter avec Google
            </MdFilledButton>
          </div>
        )}
      </MdOutlinedCard>

      {/* Firestore Sync & Storage Split */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(380px, 1fr))", gap: 16 }}>
        {/* Firestore logs */}
        <MdOutlinedCard style={{ padding: 20 }}>
          <h3 style={{ margin: "0 0 16px" }} className="owui-row">
            <MdIcon style={{ marginRight: 8, color: "#FFCA28" }}>sync</MdIcon>
            Synchronisation Temps Réel (Cloud Firestore)
          </h3>
          
          <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
            <div style={{ flex: 1 }}>
              <MdOutlinedTextField
                label="Message test à synchroniser"
                value={newDocText}
                onInput={(e) => setNewDocText((e.target as HTMLInputElement).value)}
                style={{ width: "100%" }}
              />
            </div>
            <MdFilledTonalButton onClick={handleAddSyncDoc} disabled={syncBusy || !newDocText.trim()}>
              Envoyer
            </MdFilledTonalButton>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 8, maxHeight: 200, overflowY: "auto" }}>
            {syncDocs === undefined ? (
              <div style={{ display: "flex", justifyContent: "center", padding: 16 }}>
                <MdCircularProgress indeterminate />
              </div>
            ) : syncDocs.length === 0 ? (
              <p className="owui-muted" style={{ margin: 0, textAlign: "center" }}>Aucun document synchronisé. Envoyez-en un !</p>
            ) : (
              syncDocs.map((d) => (
                <div
                  key={d.id}
                  className="owui-spread"
                  style={{
                    padding: "8px 12px",
                    borderRadius: 8,
                    background: "var(--md-sys-color-surface-container-low)",
                    alignItems: "center"
                  }}
                >
                  <span style={{ fontSize: 14 }}>{d.val}</span>
                  <button
                    onClick={() => handleDeleteSyncDoc(d.id)}
                    style={{ border: 0, background: "transparent", cursor: "pointer", color: "var(--md-sys-color-error)", display: "flex" }}
                  >
                    <MdIcon style={{ fontSize: 18 }}>delete</MdIcon>
                  </button>
                </div>
              ))
            )}
          </div>
        </MdOutlinedCard>

        {/* Cloud Storage */}
        <MdOutlinedCard style={{ padding: 20 }}>
          <h3 style={{ margin: "0 0 16px" }} className="owui-row">
            <MdIcon style={{ marginRight: 8, color: "#4285F4" }}>folder_open</MdIcon>
            Stockage Cloud Multimodal (Cloud Storage)
          </h3>

          <div style={{ display: "flex", flexDirection: "column", gap: 12, marginBottom: 16 }}>
            <div className="owui-row" style={{ gap: 8, alignItems: "center" }}>
              <input
                type="file"
                id="file-upload-input"
                style={{ display: "none" }}
                onChange={(e) => setSelectedFile(e.target.files?.[0] || null)}
              />
              <label htmlFor="file-upload-input" style={{ cursor: "pointer" }}>
                <div
                  className="owui-row"
                  style={{
                    padding: "0 24px",
                    height: 40,
                    borderRadius: 20,
                    border: "1px solid var(--md-sys-color-outline)",
                    alignItems: "center",
                    justifyContent: "center",
                    gap: 8,
                    width: "fit-content",
                    color: "var(--md-sys-color-primary)",
                    fontSize: 14,
                    fontWeight: 500,
                  }}
                >
                  <MdIcon style={{ fontSize: 18 }}>attach_file</MdIcon>
                  Choisir un fichier
                </div>
              </label>
              {selectedFile && <span style={{ fontSize: 13 }}>{selectedFile.name}</span>}
            </div>

            {selectedFile && (
              <MdFilledButton onClick={handleUploadFile}>
                Téléverser sur Cloud Storage
              </MdFilledButton>
            )}

            {uploadProgress !== null && (
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <MdCircularProgress value={uploadProgress / 100} />
                <span style={{ fontSize: 13 }}>Téléversement: {Math.round(uploadProgress)}%</span>
              </div>
            )}
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 8, maxHeight: 200, overflowY: "auto" }}>
            {files.length === 0 ? (
              <p className="owui-muted" style={{ margin: 0, textAlign: "center" }}>Aucun fichier téléversé pour le moment.</p>
            ) : (
              files.map((f) => (
                <div
                  key={f.name}
                  className="owui-spread"
                  style={{
                    padding: "8px 12px",
                    borderRadius: 8,
                    background: "var(--md-sys-color-surface-container-low)",
                    alignItems: "center"
                  }}
                >
                  <span
                    style={{
                      fontSize: 13,
                      textOverflow: "ellipsis",
                      overflow: "hidden",
                      whiteSpace: "nowrap",
                      maxWidth: "75%"
                    }}
                  >
                    {f.name}
                  </span>
                  <a
                    href={f.url}
                    target="_blank"
                    rel="noreferrer"
                    style={{ textDecoration: "none", color: "var(--md-sys-color-primary)", display: "flex" }}
                  >
                    <MdIcon style={{ fontSize: 18 }}>open_in_new</MdIcon>
                  </a>
                </div>
              ))
            )}
          </div>
        </MdOutlinedCard>
      </div>
    </div>
  );
}
