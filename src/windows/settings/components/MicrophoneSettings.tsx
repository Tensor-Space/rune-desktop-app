import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, CheckCircle2, Loader2 } from "lucide-react";

export const MicrophoneSettings = () => {
  const [hasPermission, setHasPermission] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isChecking, setIsChecking] = useState<boolean>(true);
  const [isRequesting, setIsRequesting] = useState<boolean>(false);

  useEffect(() => {
    checkPermissions();
  }, []);

  const checkPermissions = async () => {
    setIsChecking(true);
    setError(null);

    try {
      // Use the Tauri command to check microphone permissions
      const permitted: boolean = await invoke("check_microphone_permissions");
      setHasPermission(permitted);

      if (!permitted) {
        setError("Microphone access is not granted");
      }
    } catch (err) {
      console.error("Error checking microphone permissions:", err);
      setError(`Failed to check microphone permissions: ${err}`);
      setHasPermission(false);
    } finally {
      setIsChecking(false);
    }
  };

  const requestPermissions = async () => {
    setIsRequesting(true);
    setError(null);

    try {
      // Use the Tauri command to request microphone permissions
      const granted: boolean = await invoke("request_microphone_permissions");

      setHasPermission(granted);

      if (!granted) {
        setError("Microphone access was denied");
      }
    } catch (err) {
      console.error("Error requesting microphone permissions:", err);
      setError(`Failed to request microphone permissions: ${err}`);
      setHasPermission(false);
    } finally {
      setIsRequesting(false);
    }
  };

  return (
    <Card className="bg-card">
      <CardHeader>
        <CardTitle>Microphone Permissions</CardTitle>
        <CardDescription>
          Required for voice recording and audio input functionality
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {isChecking ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-6 w-6 animate-spin text-primary" />
            <span className="ml-2">Checking permissions...</span>
          </div>
        ) : error ? (
          <div className="space-y-4">
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>

            <Button
              onClick={requestPermissions}
              disabled={isRequesting}
              className="bg-primary hover:bg-primary/90 w-full"
            >
              {isRequesting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Requesting Permissions...
                </>
              ) : (
                "Grant Microphone Permissions"
              )}
            </Button>
          </div>
        ) : hasPermission ? (
          <Alert className="bg-green-50 border-green-200">
            <CheckCircle2 className="h-5 w-5 text-green-500" />
            <AlertDescription className="text-green-700 ml-2">
              Microphone access is granted
            </AlertDescription>
          </Alert>
        ) : (
          <div className="space-y-4">
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                Microphone access is required for voice recording
              </AlertDescription>
            </Alert>

            <Button
              onClick={requestPermissions}
              disabled={isRequesting}
              className="bg-primary hover:bg-primary/90 w-full"
            >
              {isRequesting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Requesting Permissions...
                </>
              ) : (
                "Grant Microphone Permissions"
              )}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
