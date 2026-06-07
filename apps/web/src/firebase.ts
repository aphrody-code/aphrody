// SPDX-License-Identifier: Apache-2.0

import { initializeApp } from "firebase/app";
import { getAnalytics, isSupported, Analytics } from "firebase/analytics";
import { getAuth } from "firebase/auth";
import { getFirestore } from "firebase/firestore";
import { getStorage } from "firebase/storage";

// Browser-safe env access. `process` does not exist in the bundled browser
// runtime, so reading `process.env.*` directly throws "process is not defined"
// at module-eval time and crashes the entire app on load. Guard it; the values
// below are the public Firebase web config (safe to ship) used as defaults, and
// an optional `process.env` override still applies when bundled under Bun.
const env: Record<string, string | undefined> =
  typeof process !== "undefined" && process.env ? process.env : {};

// Your web app's Firebase configuration
const firebaseConfig = {
  apiKey: env.FIREBASE_API_KEY || "AIzaSyAi4UFBVSstkMGYQVQdYJTtJ_mVYQgKOIk",
  authDomain: env.FIREBASE_AUTH_DOMAIN || "aphrody.firebaseapp.com",
  projectId: env.FIREBASE_PROJECT_ID || "aphrody",
  storageBucket: env.FIREBASE_STORAGE_BUCKET || "aphrody.firebasestorage.app",
  messagingSenderId: env.FIREBASE_MESSAGING_SENDER_ID || "468000409790",
  appId: env.FIREBASE_APP_ID || "1:468000409790:web:d179b857bda9ede592b6ef",
  measurementId: env.FIREBASE_MEASUREMENT_ID || "G-RM8TMT18F3"
};

// Initialize Firebase
const app = initializeApp(firebaseConfig);

// Initialize Services
const auth = getAuth(app);
const db = getFirestore(app);
const storage = getStorage(app);

// Initialize Analytics conditionally
let analytics: Analytics | null = null;

if (typeof window !== "undefined") {
  isSupported().then((supported) => {
    if (supported) {
      analytics = getAnalytics(app);
    }
  }).catch((err) => {
    console.error("Firebase Analytics initialization error:", err);
  });
}

export { app, auth, db, storage, analytics };
