export interface NewsItem {
  title: string;
  url: string;
  source: string;
  date: string;
  image: string;
}

/** Curated articles with confirmed cover images, sorted newest first */
const CURATED: NewsItem[] = [
  {
    date: "25 Sep 2025",
    source: "The Unstumbled",
    title: "Top 10 Best Durga Puja Pandals in Kolkata 2025 You Must Visit",
    image: "https://i0.wp.com/theunstumbled.com/wp-content/uploads/2025/09/best-durga-puja-pandal-in-kolkata.jpg?fit=1280%2C720&ssl=1",
    url: "https://theunstumbled.com/best-durga-puja-pandals-in-kolkata/",
  },
  {
    date: "22 Sep 2025",
    source: "The Unstumbled",
    title: "South Kolkata Durga Puja List 2025 – Best & Famous Pandal Guide",
    image: "https://i0.wp.com/theunstumbled.com/wp-content/uploads/2025/09/south-kolkata-durga-puja-list-2025.jpg?fit=1200%2C675&ssl=1",
    url: "https://theunstumbled.com/south-kolkata-durga-puja-list-2025/",
  },
  {
    date: "22 Sep 2025",
    source: "The Unstumbled",
    title: "Durga Puja 2025: Your Guide To The 7 Most Iconic Pandals in Kolkata",
    image: "https://i0.wp.com/theunstumbled.com/wp-content/uploads/2025/09/Most-Iconic-Pandals-in-Kolkata.jpg?fit=1200%2C777&ssl=1",
    url: "https://theunstumbled.com/most-iconic-pandals-kolkata-2025/",
  },
  {
    date: "13 Sep 2025",
    source: "Outlook India",
    title: "Durga Puja In Kolkata 2025: A 3-Day Itinerary For First-Timers",
    image: "https://media.assettype.com/outlookindia/2025-09-13/tod31w0p/Durga-puja?w=1200&ar=40%3A21&auto=format%2Ccompress&ogImage=true&mode=crop&enlarge=true&overlay=false&overlay_position=bottom&overlay_width=100",
    url: "https://www.outlookindia.com/brand-studio/durga-puja-in-kolkata-2025-a-3-day-itinerary-for-first-timers",
  },
  {
    date: "12 Sep 2025",
    source: "KolkataDurgotsav.com",
    title: "BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY
(DESHAPRIYA PARK) 2025 Theme & Highlights",
    image: "https://www.kolkatadurgotsav.com/wp-content/uploads/2025/09/deshapriyaparksarbojanin.jpg",
    url: "https://www.kolkatadurgotsav.com/deshapriya-park-sarbojanin-durgotsav.html",
  },
  {
    date: "27 Jul 2025",
    source: "Captured Creations",
    title: "Durga Puja 2025 Themes: Top 10 Must-Visit Pandals in Kolkata",
    image: "https://capturedcreations.in/wp-content/uploads/2025/07/07-Durga-Puja-2025.webp",
    url: "https://capturedcreations.in/durga-puja-2025-themes/",
  },
  {
    date: "18 Aug 2025",
    source: "Pujo2Pujo",
    title: "Deshapriya Park Durga Puja 2024: Where Grandeur Meets Devotion in South Kolkata",
    image: "https://www.pujo2pujo.com/wp-content/uploads/2025/08/d2.jpg",
    url: "https://www.pujo2pujo.com/deshapriya-park-durga-puja-2024-where-grandeur-meets-devotion-in-south-kolkata/",
  },
  {
    date: "20 Sep 2024",
    source: "Curly Tales",
    title: "Durga Puja 2024: Pandal Hopping Is Incomplete Without Visiting These 5 Iconic Pandals In South Kolkata",
    image: "https://curlytales.com/wp-content/uploads/2024/09/Durga-Puja-2024-Pandal-Hopping-Is-Incomplete-Without-Visiting-These-5-Iconic-Pandals-In-South-Kolkata.jpg",
    url: "https://curlytales.com/durga-puja-2024-pandal-hopping-is-incomplete-without-visiting-these-iconic-pandals-in-south-kolkata/",
  },
  {
    date: "31 Oct 2015",
    source: "Deccan Herald",
    title: "Tallest Durga Idol to Get Permanent Home",
    image: "https://media.assettype.com/deccanherald/import/sites/dh/files/article_images/2015/10/31/509254.jpg?w=1200&h=675&auto=format%2Ccompress&fit=max&enlarge=true",
    url: "https://www.deccanherald.com/india/tallest-durga-idol-get-permanent-2158685",
  },
  {
    date: "18 Oct 2015",
    source: "India TV News",
    title: "'Biggest' Durga Idol Vies for Supremacy in West Bengal",
    image: "https://resize.indiatvnews.com/en/centered/oldbucket/1200_675/mainnational/IndiaTv8abc2e_DurgaPuja.jpg",
    url: "https://www.indiatvnews.com/news/india/biggest-durga-idol-in-kolkata-deshapriya-park-55373.html",
  },
];

function parseDate(raw: string): number {
  const d = new Date(raw);
  return isNaN(d.getTime()) ? 0 : d.getTime();
}

function formatPubDate(raw: string): string {
  const d = new Date(raw);
  if (isNaN(d.getTime())) return raw;
  return d.toLocaleDateString("en-IN", { day: "2-digit", month: "short", year: "numeric" });
}

function extractXml(xml: string, tag: string): string {
  const re = new RegExp(`<${tag}(?:\\s[^>]*)?>(?:<!\\[CDATA\\[)?(.*?)(?:\\]\\]>)?</${tag}>`, "s");
  return (xml.match(re)?.[1] ?? "").trim();
}

function extractAttr(xml: string, tag: string, attr: string): string {
  const re = new RegExp(`<${tag}[^>]*\\s${attr}="([^"]*)"`, "i");
  return (xml.match(re)?.[1] ?? "").trim();
}

const RSS_QUERIES = [
  // Broad search across all sources
  '"Deshapriya Park" OR "Ballygunge Sarbojanin" Durga Puja',
  // Targeted Bengali/Kolkata outlets
  '("Deshapriya Park" OR "Ballygunge Sarbojanin" OR "Durga Puja" Kolkata) (site:anandabazar.com OR site:timesofindia.indiatimes.com OR site:telegraphindia.com OR site:bartamanpatrika.com)',
];

function parseRSSXml(xml: string): NewsItem[] {
  const itemRe = /<item>([\s\S]*?)<\/item>/g;
  const results: NewsItem[] = [];
  let m: RegExpExecArray | null;

  while ((m = itemRe.exec(xml)) !== null) {
    const block = m[1];
    const title = extractXml(block, "title");
    const link = extractXml(block, "link") || extractAttr(block, "guid", "");
    const pubDate = extractXml(block, "pubDate");
    const source = extractXml(block, "source");
    // Google News thumbnails come as <media:content url="..." medium="image"/>
    const image = extractAttr(block, "media:content", "url");

    if (title && link && image) {
      results.push({
        title,
        url: link,
        source: source || "News",
        date: formatPubDate(pubDate),
        image,
      });
    }
  }

  return results;
}

async function fetchRSSFeed(query: string): Promise<NewsItem[]> {
  const url = `https://news.google.com/rss/search?q=${encodeURIComponent(query)}&hl=en-IN&gl=IN&ceid=IN:en`;
  const res = await fetch(url, { next: { revalidate: 3600 } });
  if (!res.ok) return [];
  return parseRSSXml(await res.text());
}

async function fetchRSSNews(): Promise<NewsItem[]> {
  const feeds = await Promise.allSettled(RSS_QUERIES.map(fetchRSSFeed));
  return feeds.flatMap((r) => (r.status === "fulfilled" ? r.value : []));
}

/**
 * Returns news items: RSS results merged with curated list, deduped, newest first.
 * Falls back to curated list alone if RSS is unavailable.
 */
export async function getNews(): Promise<NewsItem[]> {
  let rss: NewsItem[] = [];
  try {
    rss = await fetchRSSNews();
  } catch {
    // RSS unavailable — curated list is the fallback
  }

  const seen = new Set<string>();
  const merged: NewsItem[] = [];

  for (const item of [...rss, ...CURATED]) {
    const key = item.url.replace(/\/$/, "");
    if (!seen.has(key)) {
      seen.add(key);
      merged.push(item);
    }
  }

  const BLOCKED_DOMAINS = ["theholidaystory.com"];

  merged.sort((a, b) => parseDate(b.date) - parseDate(a.date));
  return merged.filter(
    (item) => item.image && !BLOCKED_DOMAINS.some((d) => item.url.includes(d))
  );
}
