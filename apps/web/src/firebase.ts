// SPDX-License-Identifier: Apache-2.0

import { initializeApp } from "firebase/app";
import { getAnalytics, isSupported, Analytics } from "firebase/analytics";

// Your web app's Firebase configuration
const firebaseConfig = {
  apiKey: "AIzaSyAi4UFBVSstkMGYQVQdYJTtJ_mVYQgKOIk",
  authDomain: "aphrody.firebaseapp.com",
  projectId: "aphrody",
  storageBucket: "aphrody.firebasestorage.app",
  messagingSenderId: "468000409790",
  appId: "1:468000409790:web:d179b857bda9ede592b6ef",
  measurementId: "G-RM8TMT18F3"
};

// Initialize Firebase
const app = initializeApp(firebaseConfig);

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

export { app, analytics };
