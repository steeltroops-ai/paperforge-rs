import { Input } from "@/components/ui/input";
import { Search as SearchIcon } from "lucide-react";

export default function SearchPage() {
  return (
    <div className="flex flex-col gap-6 h-[calc(100vh-8rem)]">
      <div className="flex flex-col items-center justify-center flex-1 gap-6 max-w-2xl mx-auto w-full text-center">
        <div className="space-y-2">
          <h1 className="text-3xl font-semibold tracking-tight">
            Search Knowledge
          </h1>
          <p className="text-muted-foreground">
            Semantic search across your entire library using hybrid retrieval.
          </p>
        </div>

        <div className="relative w-full shadow-lg rounded-lg">
          <SearchIcon className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground" />
          <Input
            placeholder="Ask a question or search for a concept..."
            className="pl-12 h-14 text-lg shadow-sm border-primary/20 focus-visible:ring-primary/30"
          />
        </div>

        <div className="flex gap-2 text-xs text-muted-foreground">
          <span>Try:</span>
          <span className="bg-muted px-2 py-1 rounded-md cursor-pointer hover:bg-muted/80">
            Transformer Architecture
          </span>
          <span className="bg-muted px-2 py-1 rounded-md cursor-pointer hover:bg-muted/80">
            RAG Evaluation
          </span>
        </div>
      </div>
    </div>
  );
}
