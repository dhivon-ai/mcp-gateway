import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import { AdminApp } from "./admin/app";
import { LocalApp } from "./local/app";

const root = document.getElementById("root");
if (!root) throw new Error("Missing application root");
createRoot(root).render(
  <StrictMode>
    {document.documentElement.dataset.gatewayMode === "local" ? (
      <LocalApp />
    ) : (
      <AdminApp />
    )}
  </StrictMode>,
);
