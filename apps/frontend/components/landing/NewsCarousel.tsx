"use client";

import { useRef, useState, useEffect } from "react";
import type { NewsItem } from "@/lib/news";

const SCROLL_AMOUNT = 660; // px per arrow click (~2 cards)

export default function NewsCarousel({ items }: { items: NewsItem[] }) {
  const trackRef = useRef<HTMLDivElement>(null);
  const [canLeft, setCanLeft] = useState(false);
  const [canRight, setCanRight] = useState(false);

  function updateArrows() {
    const el = trackRef.current;
    if (!el) return;
    setCanLeft(el.scrollLeft > 4);
    setCanRight(el.scrollLeft < el.scrollWidth - el.clientWidth - 4);
  }

  useEffect(() => {
    updateArrows();
    const el = trackRef.current;
    if (!el) return;
    el.addEventListener("scroll", updateArrows, { passive: true });
    const ro = new ResizeObserver(updateArrows);
    ro.observe(el);
    return () => {
      el.removeEventListener("scroll", updateArrows);
      ro.disconnect();
    };
  }, [items]);

  function scroll(dir: "left" | "right") {
    trackRef.current?.scrollBy({
      left: dir === "left" ? -SCROLL_AMOUNT : SCROLL_AMOUNT,
      behavior: "smooth",
    });
  }

  return (
    <div className="relative group/carousel">
      {/* Left arrow */}
      <button
        aria-label="Scroll left"
        onClick={() => scroll("left")}
        className={[
          "absolute left-0 top-1/2 -translate-y-1/2 z-10 -translate-x-1/2",
          "w-10 h-10 rounded-full bg-white border border-gray-200 shadow-md",
          "flex items-center justify-center text-gray-600 hover:text-orange-600",
          "transition-all duration-200",
          canLeft ? "opacity-100 pointer-events-auto" : "opacity-0 pointer-events-none",
        ].join(" ")}
      >
        <svg xmlns="http://www.w3.org/2000/svg" className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
        </svg>
      </button>

      {/* Right arrow */}
      <button
        aria-label="Scroll right"
        onClick={() => scroll("right")}
        className={[
          "absolute right-0 top-1/2 -translate-y-1/2 z-10 translate-x-1/2",
          "w-10 h-10 rounded-full bg-white border border-gray-200 shadow-md",
          "flex items-center justify-center text-gray-600 hover:text-orange-600",
          "transition-all duration-200",
          canRight ? "opacity-100 pointer-events-auto" : "opacity-0 pointer-events-none",
        ].join(" ")}
      >
        <svg xmlns="http://www.w3.org/2000/svg" className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
        </svg>
      </button>

      {/* Scrollable track */}
      <div
        ref={trackRef}
        className="flex gap-5 overflow-x-auto pb-3 scroll-smooth"
        style={{ scrollSnapType: "x mandatory", msOverflowStyle: "none", scrollbarWidth: "none" }}
      >
        {/* hide webkit scrollbar via inline style since Tailwind doesn't ship scrollbar-hide by default */}
        <style>{`.news-track::-webkit-scrollbar{display:none}`}</style>

        {items.map((item) => (
          <a
            key={item.url}
            href={item.url}
            target="_blank"
            rel="noopener noreferrer"
            className="group flex-none w-72 rounded-xl border border-gray-100 bg-white shadow-sm hover:shadow-lg transition-shadow overflow-hidden flex flex-col"
            style={{ scrollSnapAlign: "start" }}
          >
            {/* Cover photo */}
            <div className="relative w-full h-44 bg-orange-50 overflow-hidden">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={item.image}
                alt={item.title}
                className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                loading="lazy"
                onError={(e) => {
                  const img = e.currentTarget as HTMLImageElement;
                  if (!img.dataset.fallback) {
                    img.dataset.fallback = "1";
                    img.src = "https://i0.wp.com/theunstumbled.com/wp-content/uploads/2025/09/best-durga-puja-pandal-in-kolkata.jpg?fit=1280%2C720&ssl=1";
                  }
                }}
              />
            </div>

            {/* Accent bar */}
            <div className="h-1 bg-gradient-to-r from-orange-500 to-amber-400 flex-none" />

            {/* Text */}
            <div className="p-4 flex flex-col flex-1">
              <div className="flex items-center gap-2 mb-2">
                <span className="inline-block px-2 py-0.5 rounded-full text-xs font-semibold bg-orange-100 text-orange-700 truncate max-w-[120px]">
                  {item.source}
                </span>
                <span className="text-xs text-gray-400 whitespace-nowrap">{item.date}</span>
              </div>
              <h3 className="font-bold text-gray-900 text-sm leading-snug group-hover:text-orange-700 transition-colors line-clamp-3 flex-1">
                {item.title}
              </h3>
              <span className="mt-3 text-xs text-orange-600 font-medium">Read article →</span>
            </div>
          </a>
        ))}
      </div>
    </div>
  );
}
