export type Paper = {
  id: string;
  title: string;
  authors: string[];
  publishedDate: Date;
  doi?: string;
  fileSize: number;
  tags: string[];
  status: "processing" | "embedded" | "failed";
  abstract?: string;
};

// Mock data (temporary)
export const mockPapers: Paper[] = [
  {
    id: "1",
    title: "Attention Is All You Need",
    authors: ["Vaswani et al."],
    publishedDate: new Date("2017-06-12"),
    fileSize: 2450000,
    tags: ["NLP", "Transformers"],
    status: "embedded",
    abstract:
      "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks...",
  },
  {
    id: "2",
    title:
      "BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding",
    authors: ["Devlin et al."],
    publishedDate: new Date("2018-10-11"),
    fileSize: 1800000,
    tags: ["NLP", "Deep Learning"],
    status: "processing",
    abstract: "We introduce a new language representation model called BERT...",
  },
];
