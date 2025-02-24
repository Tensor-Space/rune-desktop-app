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
import { useState } from "react";

const AVAILABLE_MODELS = [
  { id: "gpt-4", name: "GPT-4" },
  { id: "gpt-3.5-turbo", name: "GPT-3.5 Turbo" },
  { id: "claude-2", name: "Claude 2" },
  { id: "claude-instant", name: "Claude Instant" },
];

export const AIModelSettings = () => {
  const [selectedModel, setSelectedModel] = useState("gpt-4");
  const [error, setError] = useState<string | null>(null);

  const handleModelChange = async (modelId: string) => {
    try {
      // Add your model change logic here
      setSelectedModel(modelId);
      setError(null);
    } catch (error) {
      setError("Failed to change AI model");
      console.error("Model change error:", error);
    }
  };

  return (
    <Card className="bg-card">
      <CardHeader>
        <CardTitle>AI Model Selection</CardTitle>
        <CardDescription>Choose your preferred AI model</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <Select value={selectedModel} onValueChange={handleModelChange}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder="Select an AI model" />
          </SelectTrigger>
          <SelectContent>
            {AVAILABLE_MODELS.map((model) => (
              <SelectItem key={model.id} value={model.id}>
                {model.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <div className="mt-4 text-sm text-muted-foreground">
          <p>
            The selected model will be used for all AI interactions within the
            app.
          </p>
        </div>
      </CardContent>
    </Card>
  );
};
