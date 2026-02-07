export default function GraphPage() {
  return (
    <div className="flex h-[calc(100vh-6rem)] w-full items-center justify-center border-dashed border-2 border-muted-foreground/20 rounded-xl bg-muted/5 relative overflow-hidden group">
      <div className="absolute inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:24px_24px] pointer-events-none" />

      <div className="flex flex-col items-center gap-2">
        <div className="h-12 w-12 rounded-full bg-primary/20 backdrop-blur border border-primary/40 animate-pulse flex items-center justify-center">
          <div className="h-3 w-3 bg-primary rounded-full shadow-[0_0_12px_rgba(37,99,235,0.6)]" />
        </div>
        <h2 className="text-xl font-medium tracking-tight mt-4">
          Knowledge Graph Visualization
        </h2>
        <p className="text-muted-foreground text-sm max-w-xs text-center">
          This feature will map citation networks and concept clusters using
          Force-Directed Layouts.
        </p>
        <span className="mt-4 px-3 py-1 bg-amber-500/10 text-amber-500 text-xs font-mono rounded-full border border-amber-500/20">
          COMING SOON
        </span>
      </div>
    </div>
  );
}
