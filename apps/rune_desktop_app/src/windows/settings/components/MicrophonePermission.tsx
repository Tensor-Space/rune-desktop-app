import { useState, useEffect } from "react";
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

interface MicrophonePermissionProps {
  onPermissionChange?: (permitted: boolean) => void;
}

export const MicrophonePermission = ({
  onPermissionChange,
}: MicrophonePermissionProps) => {
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
      // Check if the browser supports getUserMedia
      if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
        throw new Error("Browser doesn't support audio input");
      }

      // Directly try to access the microphone instead of using permissions API
      // This is more reliable across browsers, especially WebKit
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });

      // If we get here, permission is granted
      setHasPermission(true);
      onPermissionChange?.(true);

      // Clean up
      stream.getTracks().forEach((track) => track.stop());
    } catch (err) {
      console.error("Error checking microphone permissions:", err);

      // Check if this is a permission error
      if (
        err instanceof DOMException &&
        (err.name === "NotAllowedError" || err.name === "PermissionDeniedError")
      ) {
        setError("Microphone access is blocked or denied");
      } else {
        setError(
          `Failed to access microphone: ${err instanceof Error ? err.message : String(err)}`,
        );
      }
      setHasPermission(false);
      onPermissionChange?.(false);
    } finally {
      setIsChecking(false);
    }
  };

  const requestPermissions = async () => {
    setIsRequesting(true);
    setError(null);

    try {
      // Request microphone permission by attempting to access the microphone
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });

      // If we got here, permission was granted
      setHasPermission(true);
      onPermissionChange?.(true);

      // Stop all audio tracks to clean up
      stream.getTracks().forEach((track) => track.stop());
    } catch (err) {
      console.error("Error requesting microphone permissions:", err);
      setError(
        `Microphone access was denied: ${err instanceof Error ? err.message : String(err)}`,
      );
      setHasPermission(false);
      onPermissionChange?.(false);
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
          <Alert>
            <CheckCircle2 className="h-5 w-5 text-green-500" />
            <AlertDescription>Microphone access is granted</AlertDescription>
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
