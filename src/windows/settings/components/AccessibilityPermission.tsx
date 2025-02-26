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
import { AlertCircle, CheckCircle2 } from "lucide-react";

interface AccessibilityPermissionProps {
  onPermissionChange?: (permitted: boolean) => void;
}

export const AccessibilityPermission = ({
  onPermissionChange,
}: AccessibilityPermissionProps) => {
  const [hasPermission, setHasPermission] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    checkPermissions();
  }, []);

  const checkPermissions = async () => {
    try {
      const permitted = await invoke<boolean>(
        "check_accessibility_permissions",
      );
      setHasPermission(permitted);
      onPermissionChange?.(permitted);
      setError(null);
    } catch (error) {
      setError("Failed to check accessibility permissions");
      console.error("Permission check error:", error);
    }
  };

  const requestPermissions = async () => {
    try {
      const granted = await invoke<boolean>(
        "request_accessibility_permissions",
      );
      setHasPermission(granted);
      onPermissionChange?.(granted);
      setError(null);
    } catch (error) {
      setError("Failed to request accessibility permissions");
      console.error("Permission request error:", error);
    }
  };

  return (
    <Card className="bg-card">
      <CardHeader>
        <CardTitle>Accessibility Permissions</CardTitle>
        <CardDescription>
          Required for global keyboard shortcuts and system-wide functionality
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : hasPermission ? (
          <Alert>
            <CheckCircle2 className="h-4 w-4 text-green-500" />
            <AlertDescription>
              Accessibility permissions are granted
            </AlertDescription>
          </Alert>
        ) : (
          <div className="space-y-4">
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                Accessibility permissions are required for full functionality
              </AlertDescription>
            </Alert>
            <Button onClick={requestPermissions}>Grant Permissions</Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
