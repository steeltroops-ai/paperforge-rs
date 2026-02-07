"use client";

import { ColumnDef } from "@tanstack/react-table";
import { Paper } from "../types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { MoreHorizontal, FileText, Download } from "lucide-react";

export const columns: ColumnDef<Paper>[] = [
  {
    accessorKey: "title",
    header: "Title",
    cell: ({ row }) => {
      const paper = row.original;
      return (
        <div className="flex items-center gap-2 group-hover:text-primary transition-colors cursor-pointer">
          <FileText className="h-4 w-4 text-muted-foreground mr-1" />
          <span className="font-semibold truncate max-w-[300px]">
            {paper.title}
          </span>
        </div>
      );
    },
  },
  {
    accessorKey: "authors",
    header: "Authors",
    cell: ({ row }) => {
      return (
        <div className="text-muted-foreground text-xs truncate max-w-[150px]">
          {row.original.authors.join(", ")}
        </div>
      );
    },
  },
  {
    accessorKey: "tags",
    header: "Tags",
    cell: ({ row }) => {
      const tags = row.original.tags as string[];
      return (
        <div className="flex gap-1 flex-wrap">
          {tags.slice(0, 2).map((tag) => (
            <Badge
              key={tag}
              variant="secondary"
              className="text-[10px] px-1.5 py-0.5 font-normal bg-sidebar-accent text-sidebar-foreground border-sidebar-border"
            >
              {tag}
            </Badge>
          ))}
          {tags.length > 2 && (
            <span className="text-[10px] text-muted-foreground self-center">
              +{tags.length - 2}
            </span>
          )}
        </div>
      );
    },
  },
  {
    accessorKey: "publishedDate",
    header: "Date",
    cell: ({ row }) => {
      const date = row.original.publishedDate as Date;
      return (
        <span className="text-xs text-muted-foreground font-mono">
          {date.toLocaleDateString()}
        </span>
      );
    },
  },
  {
    accessorKey: "status",
    header: "Status",
    cell: ({ row }) => {
      const status = row.original.status as string;
      return (
        <div className="flex items-center gap-1.5">
          <div
            className={`h-1.5 w-1.5 rounded-full ${status === "embedded" ? "bg-emerald-500 shadow-[0_0_4px_rgba(16,185,129,0.4)]" : "bg-amber-400 animate-pulse"}`}
          />
          <span className="capitalize text-xs text-muted-foreground">
            {status}
          </span>
        </div>
      );
    },
  },
  {
    id: "actions",
    cell: ({ row }) => {
      return (
        <Button
          variant="ghost"
          className="h-8 w-8 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
        >
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-4 w-4" />
        </Button>
      );
    },
  },
];
