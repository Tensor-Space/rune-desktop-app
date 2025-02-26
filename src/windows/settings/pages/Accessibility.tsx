import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AccessibilityPermission } from "../components/AccessibilityPermission";

interface PermissionsProps {
  onComplete: () => void;
  isStepComplete?: boolean;
}

export const Permissions = ({ onComplete }: PermissionsProps) => {
  const [, setHasPermission] = useState(false);

  useEffect(() => {
    const checkPermissions = async () => {
      try {
        const permitted = await invoke<boolean>(
          "check_accessibility_permissions",
        );
        setHasPermission(permitted);
        if (permitted) {
          onComplete();
        }
      } catch (error) {
        console.error(error);
      }
    };

    checkPermissions();
  }, [onComplete]);

  return (
    <div className="container mx-auto">
      <AccessibilityPermission
        onPermissionChange={(permitted) => {
          setHasPermission(permitted);
          if (permitted) {
            onComplete();
          }
        }}
      />
    </div>
  );
};
