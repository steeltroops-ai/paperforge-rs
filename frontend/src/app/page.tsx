import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ArrowUpRight,
  BookOpen,
  Clock,
  FileText,
  Network,
  Search,
  Sparkles,
} from "lucide-react";
import Link from "next/link";

export default function DashboardPage() {
  return (
    <div className="flex flex-col gap-8 max-w-[1200px] mx-auto">
      {/* Hero Section */}
      <section className="relative overflow-hidden rounded-3xl border border-border/40 bg-background/50 backdrop-blur-3xl shadow-2xl shadow-primary/5 p-12 lg:p-16 text-center">
        <div className="absolute inset-0 bg-grid-pattern opacity-[0.3]" />
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[600px] h-[300px] bg-primary/20 blur-[100px] rounded-full pointer-events-none" />

        <div className="relative z-10 flex flex-col items-center gap-6">
          <div className="inline-flex items-center rounded-full border border-primary/20 bg-primary/5 px-3 py-1 text-xs font-medium text-primary mb-2">
            <Sparkles className="mr-1 h-3 w-3" />
            PaperForge Intelligence v2.0
          </div>

          <h1 className="text-4xl md:text-6xl font-bold tracking-tighter text-foreground text-balance">
            The Infinite Canvas for{" "}
            <span className="text-primary italic">Knowledge</span>
          </h1>

          <p className="max-w-[600px] text-lg text-muted-foreground md:text-xl/relaxed text-balance">
            Ingest papers, distill complex topics, and generate novel insights
            with an AI-native research companion.
          </p>

          <div className="flex flex-wrap items-center justify-center gap-4 pt-6">
            <Button
              size="lg"
              className="h-12 rounded-full px-8 text-base shadow-lg shadow-primary/25 hover:shadow-primary/40 transition-all duration-300 font-medium bg-primary text-primary-foreground hover:bg-primary/90"
              asChild
            >
              <Link href="/library">
                <FileText className="mr-2 h-4 w-4" />
                Start Reading
              </Link>
            </Button>
            <Button
              size="lg"
              variant="outline"
              className="h-12 rounded-full px-8 text-base border-primary/20 bg-background/50 backdrop-blur-sm hover:bg-primary/5 hover:border-primary/40 transition-all duration-300"
              asChild
            >
              <Link href="/search">
                <Search className="mr-2 h-4 w-4 text-primary" />
                Search Concepts
              </Link>
            </Button>
          </div>
        </div>
      </section>

      {/* Quick Stats Grid */}
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        {[
          {
            title: "Total Papers",
            value: "1,204",
            footer: "+12 from last week",
            icon: BookOpen,
          },
          {
            title: "Reading Time",
            value: "12h 45m",
            footer: "+2.5h this week",
            icon: Clock,
          },
          {
            title: "Knowledge Graph",
            value: "854",
            footer: "nodes connected",
            icon: Network,
          },
          {
            title: "Pending Review",
            value: "3",
            footer: "papers queued",
            icon: FileText,
          },
        ].map((stat, i) => (
          <Card
            key={i}
            className="glass-card shadow-sm hover:shadow-md border-border/40"
          >
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                {stat.title}
              </CardTitle>
              <stat.icon className="h-4 w-4 text-primary/70" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold font-mono tracking-tight text-foreground">
                {stat.value}
              </div>
              <p className="text-xs text-muted-foreground mt-1">
                {stat.footer}
              </p>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Recent Activity */}
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-7">
        <Card className="col-span-4 glass-card border-border/40">
          <CardHeader>
            <CardTitle className="text-lg">Recent Papers</CardTitle>
            <CardDescription>You ingested 4 papers this week.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col gap-3">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="group flex items-center justify-between p-3 rounded-lg border border-transparent hover:border-border/60 hover:bg-muted/50 transition-all duration-200 cursor-pointer"
                >
                  <div className="flex items-start gap-4 overflow-hidden">
                    <div className="h-10 w-10 shrink-0 rounded-lg bg-primary/10 border border-primary/20 grid place-items-center text-[10px] font-bold text-primary">
                      PDF
                    </div>
                    <div className="grid gap-1">
                      <p className="font-medium text-sm truncate group-hover:text-primary transition-colors">
                        Attention is All You Need
                      </p>
                      <p className="text-xs text-muted-foreground">
                        Vaswani et al. • 2017
                      </p>
                    </div>
                  </div>
                  <ArrowUpRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-all -translate-x-2 group-hover:translate-x-0" />
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card className="col-span-3 glass-card border-border/40 bg-gradient-to-b from-primary/5 to-transparent">
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <Sparkles className="h-4 w-4 text-primary" />
              Copilot Insights
            </CardTitle>
            <CardDescription>AI-generated connections</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="p-4 rounded-xl bg-background/40 border border-border/50 backdrop-blur-sm shadow-sm">
              <div className="flex items-center gap-2 mb-2 text-primary font-medium text-sm">
                <Sparkles className="h-3 w-3" />
                New Theme Detected
              </div>
              <p className="text-muted-foreground text-sm leading-relaxed">
                Your recent collection focuses heavily on{" "}
                <span className="text-foreground font-medium">
                  "Efficient Transformers"
                </span>
                . Consider exploring{" "}
                <span className="text-foreground font-medium underline decoration-primary/30 underline-offset-4">
                  "Linear Attention Mechanisms"
                </span>{" "}
                next.
              </p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
