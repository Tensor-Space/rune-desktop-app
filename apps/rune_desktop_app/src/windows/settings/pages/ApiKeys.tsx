import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ApiKeyForm } from "../components/ApiKeyForm";
import { Settings } from "../types";

interface ApiKeysProps {
  onComplete: () => void;
  isStepComplete?: boolean;
}

export const ApiKeys = ({ onComplete }: ApiKeysProps) => {
  const [, setHasApiKey] = useState(false);

  useEffect(() => {
    const checkApiKeys = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        const hasKey = !!settings.api_keys.openai;
        setHasApiKey(hasKey);
        if (hasKey) {
          onComplete();
        }
      } catch (error) {
        console.error(error);
      }
    };

    checkApiKeys();
  }, [onComplete]);

  return (
    <div className="container mx-auto">
      <ApiKeyForm
        onKeySet={(keySet) => {
          setHasApiKey(keySet);
          if (keySet) {
            onComplete();
          }
        }}
      />
    </div>
  );
};
