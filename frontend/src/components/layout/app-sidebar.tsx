"use client";

import {
  BookOpen,
  Command,
  Frame,
  Library,
  Network,
  Search,
  Settings2,
  Sparkles,
  SquareTerminal,
} from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";
import * as React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";

// Data for sidebar
const data = {
  user: {
    name: "Dr. Researcher",
    email: "research@paperforge.ai",
    avatar: "/avatars/shadcn.jpg",
  },
  navMain: [
    {
      title: "Discovery",
      url: "#",
      icon: SquareTerminal,
      isActive: true,
      items: [
        {
          title: "Dashboard",
          url: "/",
          icon: Frame,
        },
        {
          title: "Search Knowledge",
          url: "/search",
          icon: Search,
        },
        {
          title: "Ask Copilot",
          url: "/chat",
          icon: Sparkles,
        },
      ],
    },
    {
      title: "Research Intelligence",
      url: "#",
      icon: BookOpen,
      items: [
        {
          title: "My Library",
          url: "/library",
          icon: Library,
        },
        {
          title: "Graph View",
          url: "/graph",
          icon: Network,
        },
      ],
    },
  ],
  footer: [
    {
      title: "Settings",
      url: "/settings",
      icon: Settings2,
    },
  ],
};

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const pathname = usePathname();

  return (
    <Sidebar
      collapsible="icon"
      className="border-r border-sidebar-border"
      {...props}
    >
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild>
              <Link href="/">
                <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground shadow-sm ring-1 ring-white/10">
                  <Command className="size-4" />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-semibold tracking-tight">
                    PaperForge
                  </span>
                  <span className="truncate text-xs text-muted-foreground">
                    Pro Research
                  </span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        {data.navMain.map((group) => (
          <SidebarGroup key={group.title}>
            <SidebarGroupLabel>{group.title}</SidebarGroupLabel>
            <SidebarMenu>
              {group.items.map((item) => {
                const isActive = pathname === item.url;
                return (
                  <SidebarMenuItem key={item.title}>
                    <SidebarMenuButton
                      asChild
                      isActive={isActive}
                      tooltip={item.title}
                      className={cn(
                        "transition-all duration-200",
                        isActive &&
                          "bg-sidebar-accent font-medium text-sidebar-accent-foreground shadow-sm icon-effect-glow",
                      )}
                    >
                      <Link href={item.url}>
                        <item.icon
                          className={cn("size-4", isActive && "text-primary")}
                        />
                        <span>{item.title}</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroup>
        ))}
      </SidebarContent>

      <SidebarFooter>
        <SidebarMenu>
          {data.footer.map((item) => (
            <SidebarMenuItem key={item.title}>
              <SidebarMenuButton asChild tooltip={item.title}>
                <Link href={item.url}>
                  <item.icon className="text-muted-foreground" />
                  <span>{item.title}</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
