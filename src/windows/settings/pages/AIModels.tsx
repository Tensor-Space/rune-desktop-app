import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

export const AIModels = () => {
  return (
    <div className="space-y-8">
      <h1 className="text-2xl font-bold text-foreground">AI Models</h1>
      <Card className="bg-card">
        <CardHeader>
          <CardTitle>Model Settings</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground">
            AI model settings will be available soon.
          </p>
        </CardContent>
      </Card>
    </div>
  );
};
