import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { installNativeShell } from "./app/nativeShell";
import "./styles.css";

installNativeShell();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
