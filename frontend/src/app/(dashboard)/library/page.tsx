"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { UploadCloud, FolderOpen, Loader2 } from "lucide-react";

import { DataTable } from "@/components/ui/data-table";
import { columns } from "@/features/library/components/columns";
import { mockPapers, Paper } from "@/features/library/types";
import { useState } from "react";

export default function LibraryPage({
  searchParams,
}: {
  searchParams: Promise<{ [key: string]: string | string[] | undefined }>;
}) {
  const [data, setData] = useState<Paper[]>(mockPapers);

  return (
    <div className="flex flex-col gap-6">
      {/* Header */}
      <div className="flex items-center justify-between border-b pb-4 border-border/40">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight">Library</h1>
          <p className="text-muted-foreground text-sm flex items-center gap-2 mt-1">
            <FolderOpen className="h-4 w-4" />
            {data.length} papers indexed • 1.2 GB stored
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            className="border-dashed border-sidebar-border bg-sidebar/50 hover:bg-sidebar"
          >
            <UploadCloud className="mr-2 h-4 w-4" />
            Ingest Paper
          </Button>
          <Button>Add Collection</Button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="grid gap-6 lg:grid-cols-4">
        {/* Sidebar Navigation for Library (Filters) */}
        <div className="lg:col-span-1 space-y-4">
          <Card className="shadow-none border-border/60 bg-sidebar/20">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Collections</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-1 px-2 pb-2">
              {[
                "All Papers",
                "Neural Networks",
                "Bioinformatics",
                "Unsorted",
              ].map((item) => (
                <div
                  key={item}
                  className="flex items-center justify-between px-3 py-2 rounded-md hover:bg-sidebar-accent hover:text-sidebar-accent-foreground text-sm cursor-pointer transition-colors group"
                >
                  <span className="font-medium text-muted-foreground group-hover:text-foreground">
                    {item}
                  </span>
                  <span className="text-xs text-muted-foreground bg-sidebar-border px-1.5 rounded-sm">
                    12
                  </span>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card className="shadow-none border-border/60 bg-sidebar/20">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Tags</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-2 px-4 pb-4">
              {["NLP", "CV", "RL", "Systems", "Rust", "GPU", "Math"].map(
                (tag) => (
                  <span
                    key={tag}
                    className="text-xs bg-sidebar-border text-sidebar-foreground px-2 py-1 rounded-md border border-sidebar-ring/20 hover:border-sidebar-ring cursor-pointer transition-colors"
                  >
                    #{tag}
                  </span>
                ),
              )}
            </CardContent>
          </Card>
        </div>

        {/* Data Table */}
        <div className="lg:col-span-3">
          <DataTable columns={columns} data={data} />
        </div>
      </div>
    </div>
  );
}
