import { Button } from "@/components/ui/button";
import { Send, Sparkles } from "lucide-react";

export default function ChatPage() {
  return (
    <div className="flex flex-col h-[calc(100vh-6rem)] relative">
      <div className="flex-1 flex flex-col justify-center items-center text-center gap-6 p-4">
        <div className="p-4 rounded-full bg-primary/10 text-primary animate-pulse">
          <Sparkles className="h-8 w-8" />
        </div>
        <h1 className="text-2xl font-semibold tracking-tight">
          Copilot Assistant
        </h1>
        <p className="text-muted-foreground max-w-md">
          Ask questions about your research library. The copilot can synthesize
          information from multiple papers.
        </p>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 w-full max-w-2xl text-left">
          <Button
            variant="outline"
            className="h-auto p-4 flex flex-col items-start gap-1 hover:bg-muted/50"
          >
            <span className="font-medium text-sm">Summarize key findings</span>
            <span className="text-xs text-muted-foreground w-full truncate">
              Create a summary of the latest RL architectures...
            </span>
          </Button>
          <Button
            variant="outline"
            className="h-auto p-4 flex flex-col items-start gap-1 hover:bg-muted/50"
          >
            <span className="font-medium text-sm">
              Draft a literature review
            </span>
            <span className="text-xs text-muted-foreground w-full truncate">
              Using tags "Transformer" and "Efficient"...
            </span>
          </Button>
        </div>
      </div>

      {/* Input Area */}
      <div className="p-4 border-t bg-background/50 backdrop-blur sticky bottom-0">
        <div className="max-w-3xl mx-auto relative flex gap-2">
          <input
            className="flex-1 bg-muted/50 border-0 rounded-xl px-4 py-3 placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary/20"
            placeholder="How does recent work on SSMs compare to attention?"
          />
          <Button size="icon" className="rounded-xl h-12 w-12 shrink-0">
            <Send className="h-5 w-5" />
          </Button>
        </div>
        <p className="text-[10px] text-center text-muted-foreground mt-2">
          AI can make mistakes. Verify important information.
        </p>
      </div>
    </div>
  );
}
