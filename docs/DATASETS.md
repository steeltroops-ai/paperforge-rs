# Academic Paper Datasets for PaperForge

This document lists recommended datasets for populating PaperForge-rs with academic papers, organized by relevance and ease of use.

## 🏆 Top Recommendations

### 1. arXiv Dataset (Best for AI/CS)

The definitive repository for Computer Science, Physics, and Math papers.

- **Content**: ~2.2M+ papers. Metadata in JSON. Full text available as PDF or LaTeX source.
- **Why it's best**:
  - High relevance for tech/AI projects.
  - "Clean" metadata structure.
  - Bulk access via Kaggle or S3.
- **Access**:
  - [Kaggle arXiv Dataset](https://www.kaggle.com/datasets/Cornell-University/arxiv) (JSON metadata)
  - [arXiv Bulk Data Access](https://info.arxiv.org/help/bulk_data/index.html) (S3 requester pays)

### 2. S2ORC (Semantic Scholar Open Research Corpus)

A massive corpus of 81.1M+ English-language academic papers spanning many disciplines.

- **Content**: Full text, abstracts, and rich citation graphs.
- **Why it's best**:
  - **Citation Graph**: Comes with a pre-built rich citation network, perfect for testing PaperForge's "Citation Propagation".
  - Parsed text: JSON format (no need to parse PDFs yourself!).
- **Access**: [AllenAI GitHub](https://github.com/allenai/s2orc) (Requests API key)

### 3. ACL Anthology (Best for NLP)

The specialized archive for Computational Linguistics and NLP.

- **Content**: ~90k papers. High quality.
- **Why it's best**:
  - Smaller, manageable size for initial testing.
  - Extremely high-quality metadata.
  - Direct PDF links.
- **Access**: [ACL Anthology](https://aclanthology.org/) (GitHub repo available)

---

## 📂 Other Notable Choices

### 4. PubMed Central (PMC) Open Access

- **Focus**: Biomedical and Life Sciences.
- **Format**: XML (highly structured, easy to parse).
- **Size**: ~5M+ articles.
- **Use Case**: If testing storage scalability or biomedical entity extraction.

### 5. CORE

- **Focus**: Aggregates open access papers from repositories worldwide.
- **Size**: 200M+ metadata records.
- **Use Case**: Massive scale testing.

### 6. DBLP

- **Focus**: Computer Science bibliography.
- **Format**: XML.
- **Use Case**: Metadata and citation analysis only (rarely contains full text).

---

## 🚀 Recommended Workflow for PaperForge

For the **Initial Prototype & Testing**:

1. **Download the [arXiv Metadata from Kaggle](https://www.kaggle.com/datasets/Cornell-University/arxiv)** (approx. 4GB JSON).
2. Filter for categories `cs.AI`, `cs.LG`, `cs.CL`.
3. Use the `id` to fetch PDFs on-demand from `export.arxiv.org` or use the bulk PDF buckets.
4. Ingest ~1,000 papers to test the pipeline.

**Why arXiv?**

- Represents the target domain (Computer Science).
- PDF layout is consistent (usually 2-column), good for testing parsers.
- Metadata includes abstract, authors, and categories, filling your DB nicely.
