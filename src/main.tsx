import React from "react";
import ReactDOM from "react-dom/client";
import "./global.css";
import { PostHogProvider } from "posthog-js/react";
import { App } from "./App";

export const REACT_APP_PUBLIC_POSTHOG_KEY =
  "phc_bmoyzLxfEIkcoUfDU3CDq9geHEEvdGblfyFSYmwuUVC";
export const REACT_APP_PUBLIC_POSTHOG_HOST = "https://us.i.posthog.com";

const options = {
  api_host: REACT_APP_PUBLIC_POSTHOG_HOST,
};

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <PostHogProvider apiKey={REACT_APP_PUBLIC_POSTHOG_KEY} options={options}>
      <App />
    </PostHogProvider>
  </React.StrictMode>,
);
