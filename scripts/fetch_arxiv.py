
import os
import time
import requests
import xml.etree.ElementTree as ET
import urllib.parse

# Configuration
QUERY = "cat:cs.AI"
MAX_RESULTS = 100
DATA_DIR = os.path.join("data", "raw")
ARXIV_API_URL = "http://export.arxiv.org/api/query"

def fetch_arxiv_papers(query, max_results=10):
    params = {
        "search_query": query,
        "start": 0,
        "max_results": max_results,
        "sortBy": "submittedDate",
        "sortOrder": "descending"
    }

    print(f"Fetching metadata for {max_results} papers from arXiv...")
    response = requests.get(ARXIV_API_URL, params=params)

    if response.status_code != 200:
        print(f"Error fetching metadata: {response.status_code}")
        return []

    root = ET.fromstring(response.content)
    papers = []

    for entry in root.findall("{http://www.w3.org/2005/Atom}entry"):
        paper = {}
        paper["title"] = entry.find("{http://www.w3.org/2005/Atom}title").text.strip()
        paper["id"] = entry.find("{http://www.w3.org/2005/Atom}id").text.split("/")[-1]
        paper["pdf_url"] = entry.find("{http://www.w3.org/2005/Atom}id").text.replace("abs", "pdf")
        papers.append(paper)

    return papers

def download_pdf(url, filepath):
    response = requests.get(url, stream=True)
    if response.status_code == 200:
        with open(filepath, 'wb') as f:
            for chunk in response.iter_content(1024):
                f.write(chunk)
        return True
    return False

def main():
    if not os.path.exists(DATA_DIR):
        os.makedirs(DATA_DIR)

    papers = fetch_arxiv_papers(QUERY, MAX_RESULTS)
    print(f"Found {len(papers)} papers. Downloading...")

    downloaded = 0
    for i, paper in enumerate(papers):
        filename = f"{paper['id']}.pdf"
        filepath = os.path.join(DATA_DIR, filename)

        if os.path.exists(filepath):
            print(f"[{i+1}/{len(papers)}] Skipping {filename} (already exists)")
            downloaded += 1
            continue

        print(f"[{i+1}/{len(papers)}] Downloading: {paper['title'][:50]}...")
        if download_pdf(paper["pdf_url"], filepath):
            downloaded += 1
            # Be nice to arXiv API
            time.sleep(3) 
        else:
            print(f"Failed to download {paper['pdf_url']}")

    print(f"\nDownload complete! {downloaded} PDFs saved to {DATA_DIR}")

if __name__ == "__main__":
    main()
