// SPDX-License-Identifier: Apache-2.0

import { initializeApp } from "firebase/app";
import { getAnalytics, isSupported, Analytics } from "firebase/analytics";
import { getAuth } from "firebase/auth";
import { getFirestore } from "firebase/firestore";
import { getStorage } from "firebase/storage";

// Your web app's Firebase configuration
const firebaseConfig = {
  apiKey: process.env.FIREBASE_API_KEY || "AIzaSyAi4UFBVSstkMGYQVQdYJTtJ_mVYQgKOIk",
  authDomain: process.env.FIREBASE_AUTH_DOMAIN || "aphrody.firebaseapp.com",
  projectId: process.env.FIREBASE_PROJECT_ID || "aphrody",
  storageBucket: process.env.FIREBASE_STORAGE_BUCKET || "aphrody.firebasestorage.app",
  messagingSenderId: process.env.FIREBASE_MESSAGING_SENDER_ID || "468000409790",
  appId: process.env.FIREBASE_APP_ID || "1:468000409790:web:d179b857bda9ede592b6ef",
  measurementId: process.env.FIREBASE_MEASUREMENT_ID || "G-RM8TMT18F3"
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
