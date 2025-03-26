import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { CheckIcon, XIcon } from "lucide-react";

interface PermissionsSetupProps {
  onPermissionGranted: () => void;
}

export const PermissionsSetup = ({
  onPermissionGranted,
}: PermissionsSetupProps) => {
  const [hasPermission, setHasPermission] = useState(true);

  const checkPermissions = async () => {
    try {
      const permitted = await invoke<boolean>(
        "check_accessibility_permissions",
      );
      setHasPermission(permitted);
      return permitted;
    } catch (error) {
      console.error(error);
      return false;
    }
  };

  useEffect(() => {
    // Check immediately on component mount
    checkPermissions();

    // Set up interval to check every second
    const intervalId = setInterval(() => {
      // Stop checking if permission is already granted
      checkPermissions().then((permitted) => {
        if (permitted) {
          clearInterval(intervalId);
        }
      });
    }, 1000);

    // Clean up interval on component unmount
    return () => clearInterval(intervalId);
  }, []);

  const requestPermission = async () => {
    try {
      await invoke("request_accessibility_permissions");
    } catch (error) {
      console.error("Failed to request permissions:", error);
    }
  };

  return (
    <div className="flex flex-col items-center">
      <h2 className="text-xl font-semibold mb-4 text-center">
        Enable Accessibility Permissions
      </h2>

      {/* Central Status Icon */}
      <div className="flex justify-center mb-6">
        <div
          className={`h-32 w-32 rounded-full flex items-center justify-center transition-all duration-300 ${
            hasPermission
              ? "bg-green-500/20 border-2 border-green-500"
              : "bg-red-500/20 border-2 border-red-500"
          }`}
        >
          {hasPermission ? (
            <CheckIcon className="h-16 w-16 text-green-500" />
          ) : (
            <XIcon className="h-16 w-16 text-red-500" />
          )}
        </div>
      </div>

      <div className="text-center mb-4">
        <span className={`text-lg font-medium `}>
          {hasPermission
            ? "Accessibility permissions granted"
            : "Accessibility permissions required"}
        </span>
      </div>

      <p className="text-neutral-400 mb-6 text-center max-w-md">
        RuneApp needs accessibility permissions to work properly. This allows us
        to paste text in the applications you are using.
      </p>

      <div className="mt-2">
        {!hasPermission ? (
          <Button size="lg" onClick={requestPermission} className="font-medium">
            Grant Permissions
          </Button>
        ) : (
          <Button
            size="lg"
            onClick={onPermissionGranted}
            className=" font-medium"
          >
            Continue
          </Button>
        )}
      </div>
    </div>
  );
};
