import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, CheckCircle2, Eye, EyeOff } from "lucide-react";
import { Settings } from "../types";

interface ApiKeyFormProps {
  onKeySet?: (hasKey: boolean) => void;
}

export const ApiKeyForm = ({ onKeySet }: ApiKeyFormProps) => {
  const [openaiKey, setOpenaiKey] = useState<string>("");
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<boolean>(false);
  const [showApiKey, setShowApiKey] = useState<boolean>(false);
  const [, setHasApiKey] = useState<boolean>(false);

  useEffect(() => {
    const loadApiKeys = async () => {
      try {
        const settings = await invoke<Settings>("get_settings");
        const hasKey = !!settings.api_keys.openai;
        setHasApiKey(hasKey);
        if (hasKey) {
          setOpenaiKey(settings.api_keys.openai || "");
          onKeySet?.(true);
        }
      } catch (error) {
        setError("Failed to load API keys");
        console.error("API key load error:", error);
      }
    };

    loadApiKeys();
  }, [onKeySet]);

  const handleSaveApiKey = async () => {
    setIsSaving(true);
    setError(null);
    setSuccess(false);

    try {
      // Validate API key format (basic check)
      if (!openaiKey.trim()) {
        setError("API key cannot be empty");
        return;
      }

      if (!openaiKey.startsWith("sk-") && openaiKey !== "[REDACTED]") {
        setError("OpenAI API key should start with 'sk-'");
        return;
      }

      await invoke("update_api_key", {
        service: "openai",
        apiKey: openaiKey.trim(),
      });

      setSuccess(true);
      setHasApiKey(true);
      onKeySet?.(true);
    } catch (error) {
      setError(`Failed to save API key: ${error}`);
      console.error("Failed to save API key:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const toggleShowApiKey = () => {
    setShowApiKey(!showApiKey);
  };

  return (
    <Card className="bg-card">
      <CardHeader>
        <CardTitle>OpenAI API Key</CardTitle>
        <CardDescription>
          Enter your OpenAI API key to use GPT models for text generation
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {success && (
          <Alert className="bg-green-500/10 border-green-500 text-green-500">
            <CheckCircle2 className="h-4 w-4" />
            <AlertDescription>API key saved successfully</AlertDescription>
          </Alert>
        )}

        <div className="space-y-1">
          <div className="relative">
            <Input
              type={showApiKey ? "text" : "password"}
              placeholder="sk-..."
              value={openaiKey}
              onChange={(e) => {
                setOpenaiKey(e.target.value);
                setSuccess(false);
              }}
              className="pr-10"
            />
            <button
              type="button"
              onClick={toggleShowApiKey}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showApiKey ? (
                <EyeOff className="h-4 w-4" />
              ) : (
                <Eye className="h-4 w-4" />
              )}
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            Your API key is stored locally and never shared.
          </p>
        </div>
      </CardContent>
      <CardFooter>
        <Button
          onClick={handleSaveApiKey}
          disabled={isSaving}
          className="w-full"
        >
          {isSaving ? "Saving..." : "Save API Key"}
        </Button>
      </CardFooter>
    </Card>
  );
};
