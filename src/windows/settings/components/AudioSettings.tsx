import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AlertCircle } from "lucide-react";
import { AudioDevice } from "../types";

export const AudioSettings = () => {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadAudioSettings = async () => {
      try {
        const audioDevices = await invoke<AudioDevice[]>("get_devices");
        setDevices(audioDevices);

        const defaultDevice = await invoke<AudioDevice | null>(
          "get_default_device",
        );
        if (defaultDevice) {
          setSelectedDevice(defaultDevice.id);
        } else if (audioDevices.length > 0) {
          setSelectedDevice(audioDevices[0].id);
        }
      } catch (error) {
        setError("Failed to load audio devices");
        console.error("Audio settings load error:", error);
      }
    };

    loadAudioSettings();
  }, []);

  const handleDeviceChange = async (deviceId: string) => {
    try {
      await invoke("set_default_device", { deviceId });
      setSelectedDevice(deviceId);
      setError(null);
    } catch (error) {
      setError("Failed to save device selection");
      console.error("Failed to set default device:", error);
    }
  };

  return (
    <Card className="bg-card">
      <CardHeader>
        <CardTitle>Audio Input</CardTitle>
        <CardDescription>Select your preferred microphone</CardDescription>
      </CardHeader>
      <CardContent>
        {error ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : (
          <Select value={selectedDevice} onValueChange={handleDeviceChange}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder="Select a microphone" />
            </SelectTrigger>
            <SelectContent>
              {devices.map((device) => (
                <SelectItem key={device.id} value={device.id}>
                  {device.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </CardContent>
    </Card>
  );
};
