import type { Metadata } from "next";
import Image from "next/image";
import Link from "next/link";
import NavBar from "@/components/landing/NavBar";
import NewsCarousel from "@/components/landing/NewsCarousel";
import { getNews } from "@/lib/news";

export const metadata: Metadata = {
  title: "BSDS",
  description:
    "One of South Kolkata's most iconic Durga Puja celebrations, founded in 1938. 87 years of tradition, heritage, and community spirit. World's tallest Durga idol — 88 feet (2015).",
};

/* ─── Data ─────────────────────────────────────────────────────────────── */


const activities = [
  {
    icon: "🏛️",
    title: "Annual Durga Puja",
    description:
      "Massive theme-based pandals with intricate artistic installations. Iconic Chandannagar-style dynamic LED, neon, and moving light displays narrating mythological tales.",
  },
  {
    icon: "🎭",
    title: "Cultural Programs",
    description:
      "Musical nights, performances by reputed artists, classical dance, and theatre on the park grounds during the five-day puja celebration.",
  },
  {
    icon: "🌿",
    title: "Eco-Friendly Initiatives",
    description:
      "Biodegradable materials, natural colours, and energy-efficient lighting — leading the way for sustainable celebrations in Kolkata.",
  },
  {
    icon: "🩸",
    title: "Blood Donation Camps",
    description:
      "Annual blood donation drives in partnership with hospitals, contributing to the city's blood bank throughout the year.",
  },
  {
    icon: "🏥",
    title: "Health Camps",
    description:
      "Free health check-up camps for underprivileged communities, dengue awareness campaigns, and medical outreach programs.",
  },
  {
    icon: "👕",
    title: "Community Service",
    description:
      "Clothing distribution drives, relief efforts during natural disasters, and year-round welfare activities for the neighbourhood.",
  },
];

const heritageTimeline = [
  {
    year: "1938",
    label: "Founded",
    detail: "BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY (DESHAPRIYA PARK) established — one of South Kolkata's earliest community pujas.",
  },
  {
    year: "2015",
    label: "Record-Breaking Idol",
    detail: "World's tallest Durga idol at 88 feet, sculpted by Mintu Pal. Listed in Limca, Indian, and Asia Books of Records. Theme: World's Tallest Durga Idol — 88 Feet.",
  },
  {
    year: "2016",
    label: "Pandal Theme",
    detail: "Idol with 1,000 Hands Slaying 100 Demons.",
  },
  {
    year: "2017",
    label: "Pandal Theme",
    detail: "Mahishmati Palace (Baahubali-inspired).",
  },
  {
    year: "2021",
    label: "UNESCO Recognition",
    detail: "Kolkata Durga Puja tradition inscribed as UNESCO Intangible Cultural Heritage of Humanity — a celebration Deshapriya Park is part of.",
  },
  {
    year: "2022",
    label: "Pandal Theme",
    detail: "Bhubaneswari.",
  },
  {
    year: "2023",
    label: "Pandal Theme",
    detail: "Jyoti.",
  },
  {
    year: "2024",
    label: "Pandal Theme",
    detail: "Universal Shakti — The Power that Connects All.",
  },
  {
    year: "2025",
    label: "87th Year",
    detail: "Boro Durga revival — revisiting the grandeur that made headlines a decade ago.",
  },
];


/* ─── Components ────────────────────────────────────────────────────────── */

function SectionHeading({
  children,
  subtitle,
}: {
  children: React.ReactNode;
  subtitle?: string;
}) {
  return (
    <div className="text-center mb-10 md:mb-14">
      <h2 className="text-2xl sm:text-3xl md:text-4xl font-bold text-gray-900 mb-3">
        {children}
      </h2>
      {subtitle && (
        <p className="text-base sm:text-lg text-gray-500 max-w-2xl mx-auto">{subtitle}</p>
      )}
    </div>
  );
}

/* ─── Page ──────────────────────────────────────────────────────────────── */

export default async function HomePage() {
  const news = await getNews();

  return (
    <>
      <NavBar />

      <main className="min-h-screen">

        {/* ── Hero ─────────────────────────────────────────────────── */}
        <section
          id="hero"
          className="relative min-h-screen flex items-center justify-center overflow-hidden"
        >
          {/* Background image */}
          <div aria-hidden="true" className="absolute inset-0">
            <Image
              src="/images/hero.png"
              alt=""
              fill
              className="object-cover object-center"
              priority
            />
          </div>
          {/* Dark overlay for text legibility */}
          <div
            aria-hidden="true"
            className="absolute inset-0 bg-gradient-to-b from-black/60 via-black/40 to-black/70"
          />

          <div className="relative z-10 text-center px-4 sm:px-8 max-w-5xl mx-auto pt-16">
            <div className="mb-6 flex justify-center">
              <Image
                src="/images/logo.jpg"
                alt="BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY (DESHAPRIYA PARK)"
                width={96}
                height={96}
                className="rounded-full object-cover drop-shadow-lg"
              />
            </div>

            <h1 className="text-2xl sm:text-3xl md:text-4xl font-extrabold text-white leading-tight drop-shadow-md mb-4 uppercase tracking-wide">
              BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY
              <br />
              <span className="text-yellow-300">(DESHAPRIYA PARK)</span>
            </h1>

            <p className="text-lg sm:text-xl md:text-2xl text-orange-100 font-medium mb-2">
              Celebrating Tradition Since 1938
            </p>
            <p className="text-sm sm:text-base text-orange-200 mb-10 max-w-2xl mx-auto">
              87 years of devotion, artistry, and community spirit — one of South Kolkata's most iconic Durga Puja celebrations.
            </p>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
              <a
                href="#activities"
                className="px-8 py-3 rounded-lg bg-white text-orange-700 font-semibold hover:bg-orange-50 transition-colors shadow-lg text-base"
              >
                Explore the Club
              </a>
              <Link
                href="/membership-form"
                className="px-8 py-3 rounded-lg bg-orange-600/80 border border-white/30 text-white font-semibold hover:bg-orange-600 transition-colors shadow-lg text-base"
              >
                Apply for Membership
              </Link>
            </div>

            {/* Scroll cue */}
            <div className="mt-16 flex flex-col items-center gap-1 text-white/60 text-xs">
              <span>Scroll to explore</span>
              <span className="animate-bounce text-lg" aria-hidden="true">↓</span>
            </div>
          </div>
        </section>

        {/* ── Club Activities ──────────────────────────────────────── */}
        <section id="activities" className="py-20 md:py-28 bg-white">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <SectionHeading subtitle="Annual celebrations, cultural programs, and community service that define Deshapriya Park.">
              Club Activities
            </SectionHeading>

            {/* Activity cards */}
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6 mb-16">
              {activities.map((act) => (
                <div
                  key={act.title}
                  className="rounded-xl border-2 border-orange-300 bg-orange-50 p-6 shadow-sm hover:shadow-lg hover:border-orange-400 hover:bg-orange-100/60 transition-all"
                >
                  <div className="text-3xl mb-3" aria-hidden="true">{act.icon}</div>
                  <h3 className="font-semibold text-gray-900 text-base mb-2">{act.title}</h3>
                  <p className="text-sm text-gray-600 leading-relaxed">{act.description}</p>
                </div>
              ))}
            </div>

          </div>
        </section>

        {/* ── History & Heritage ───────────────────────────────────── */}
        <section
          id="history"
          className="py-20 md:py-28"
          style={{
            background: "linear-gradient(180deg, #fff7ed 0%, #ffedd5 50%, #fff7ed 100%)",
          }}
        >
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <SectionHeading subtitle="Eight decades of devotion, record-breaking achievements, and UNESCO recognition.">
              History &amp; Heritage
            </SectionHeading>

            {/* Stats bar */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 sm:gap-6 mb-16">
              {[
                { value: "1938", label: "Founded" },
                { value: "87", label: "Years of Tradition" },
                { value: "88 ft", label: "Tallest Durga Idol" },
                { value: "3", label: "World Records" },
              ].map((stat) => (
                <div
                  key={stat.label}
                  className="rounded-xl bg-white border border-orange-200 p-5 text-center shadow-sm"
                >
                  <div className="text-2xl sm:text-3xl font-extrabold text-orange-600 mb-1">
                    {stat.value}
                  </div>
                  <div className="text-xs sm:text-sm text-gray-500 font-medium">{stat.label}</div>
                </div>
              ))}
            </div>

            {/* Timeline */}
            <div className="relative">
              {/* Vertical line */}
              <div
                aria-hidden="true"
                className="hidden sm:block absolute left-1/2 -translate-x-px top-0 bottom-0 w-0.5 bg-gradient-to-b from-orange-300 via-amber-400 to-orange-200"
              />

              <div className="space-y-8 sm:space-y-0">
                {heritageTimeline.map((item, i) => (
                  <div
                    key={item.year}
                    className={`relative sm:flex sm:items-start sm:gap-8 ${
                      i % 2 === 0 ? "sm:flex-row" : "sm:flex-row-reverse"
                    } mb-8 sm:mb-12`}
                  >
                    {/* Content card */}
                    <div className="sm:w-[calc(50%-2rem)] rounded-xl border border-orange-100 bg-white p-5 shadow-sm hover:shadow-md transition-shadow">
                      <div className="text-xs font-bold text-orange-500 uppercase tracking-wide mb-1">
                        {item.label}
                      </div>
                      <div className="text-2xl font-extrabold text-gray-900 mb-2">{item.year}</div>
                      <p className="text-sm text-gray-600 leading-relaxed">{item.detail}</p>
                    </div>

                    {/* Center dot */}
                    <div
                      aria-hidden="true"
                      className="hidden sm:flex absolute left-1/2 -translate-x-1/2 top-5 w-5 h-5 rounded-full bg-orange-500 border-2 border-white shadow-md items-center justify-center"
                    />

                    {/* Spacer for opposite side */}
                    <div className="hidden sm:block sm:w-[calc(50%-2rem)]" />
                  </div>
                ))}
              </div>
            </div>

            {/* UNESCO callout */}
            <div className="mt-10 rounded-2xl border border-amber-200 bg-gradient-to-r from-amber-50 to-orange-50 p-6 sm:p-8 text-center shadow-sm">
              <div className="text-3xl mb-3" aria-hidden="true">🌍</div>
              <h3 className="text-lg font-bold text-gray-800 mb-2">UNESCO Intangible Cultural Heritage</h3>
              <p className="text-sm text-gray-600 max-w-2xl mx-auto leading-relaxed">
                In 2021, the Kolkata Durga Puja tradition — of which Deshapriya Park is an integral part — was inscribed by UNESCO as an{" "}
                <strong>Intangible Cultural Heritage of Humanity</strong>, recognising its artistic, spiritual, and community significance.
              </p>
            </div>

            {/* 2015 Record callout */}
            <div className="mt-6 rounded-2xl border border-orange-200 bg-white p-6 sm:p-8 shadow-sm">
              <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
                <div className="text-4xl" aria-hidden="true">🏆</div>
                <div>
                  <h3 className="text-lg font-bold text-gray-800 mb-1">
                    World Record — 88-Foot Durga Idol (2015)
                  </h3>
                  <p className="text-sm text-gray-600 leading-relaxed">
                    In 2015, Deshapriya Park unveiled the world's tallest Durga idol at{" "}
                    <strong>88 feet</strong>, sculpted by <strong>Mintu Pal</strong> and team. The achievement was certified by the{" "}
                    <strong>Limca Book of Records</strong>, <strong>Indian Book of Records</strong>, and{" "}
                    <strong>Asia Book of Records</strong>. The scale of the celebration drew millions of visitors, making it a landmark moment in Kolkata's Durga Puja history.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* ── Latest News ──────────────────────────────────────────── */}
        <section id="news" className="py-20 md:py-28 bg-white">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <SectionHeading subtitle="The latest coverage from around the web — newest first.">
              Latest News
            </SectionHeading>

            <NewsCarousel items={news} />
          </div>
        </section>

        {/* ── Contact Information ──────────────────────────────────── */}
        <section
          id="contact"
          className="py-20 md:py-28"
          style={{
            background: "linear-gradient(180deg, #fff7ed 0%, #ffedd5 100%)",
          }}
        >
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <SectionHeading subtitle="Visit us at Deshapriya Park, Ballygunge, Kolkata.">
              Contact &amp; Location
            </SectionHeading>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-8 max-w-4xl mx-auto">
              {/* Address card */}
              <div className="rounded-2xl bg-white border border-orange-100 shadow-sm p-7">
                <div className="text-2xl mb-4" aria-hidden="true">📍</div>
                <h3 className="font-bold text-gray-900 text-base mb-4">Find Us</h3>
                <address className="not-italic text-sm text-gray-700 space-y-2">
                  <p className="font-semibold text-gray-900">
                    BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY
(DESHAPRIYA PARK)
                  </p>
                  <p>Deshapriya Park, Tilak Road</p>
                  <p>34A Manoharpukur Road, Ballygunge</p>
                  <p>Kolkata — 700029, West Bengal</p>
                </address>
                <div className="mt-5 pt-5 border-t border-orange-50 space-y-2 text-sm text-gray-600">
                  <p>
                    <span className="font-medium text-gray-700">Landmark: </span>
                    Opposite Priya Cinema, near Rash Behari Avenue
                  </p>
                  <p>
                    <span className="font-medium text-gray-700">Nearest Metro: </span>
                    Kalighat Metro Station (~1 km)
                  </p>
                </div>
              </div>

              {/* Contact details card */}
              <div className="rounded-2xl bg-white border border-orange-100 shadow-sm p-7">
                <div className="text-2xl mb-4" aria-hidden="true">📞</div>
                <h3 className="font-bold text-gray-900 text-base mb-4">Get in Touch</h3>
                <div className="space-y-4 text-sm text-gray-700">
                  <div className="flex items-center gap-3">
                    <span className="text-lg" aria-hidden="true">📱</span>
                    <div>
                      <div className="text-xs text-gray-400 font-medium uppercase tracking-wide mb-0.5">Phone / WhatsApp</div>
                      <a
                        href="tel:+919433082863"
                        className="font-semibold text-orange-600 hover:underline"
                      >
                        +91 94330 82863
                      </a>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-lg" aria-hidden="true">📘</span>
                    <div>
                      <div className="text-xs text-gray-400 font-medium uppercase tracking-wide mb-0.5">Facebook</div>
                      <a
                        href="https://www.facebook.com/Deshapriyaparkdurgotsab/"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="font-semibold text-orange-600 hover:underline"
                      >
                        Deshapriya Park Durgotsab
                      </a>
                    </div>
                  </div>
                </div>

                <div className="mt-6 pt-5 border-t border-orange-50">
                  <p className="text-xs text-gray-400 mb-4 font-medium uppercase tracking-wide">Quick Links</p>
                  <div className="flex flex-wrap gap-3">
                    <Link
                      href="/login"
                      className="px-4 py-2 rounded-lg bg-orange-500 text-white text-sm font-semibold hover:bg-orange-600 transition-colors"
                    >
                      Member Login
                    </Link>
                    <Link
                      href="/membership-form"
                      className="px-4 py-2 rounded-lg border border-orange-400 text-orange-600 text-sm font-semibold hover:bg-orange-50 transition-colors"
                    >
                      Membership Form
                    </Link>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* ── Footer ───────────────────────────────────────────────── */}
        <footer className="bg-gray-900 text-gray-300 py-10">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <div className="flex flex-col md:flex-row items-center justify-between gap-6">
              {/* Brand */}
              <div className="flex items-center gap-4 text-center md:text-left">
                <Image
                  src="/images/logo.jpg"
                  alt="BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY
(DESHAPRIYA PARK)"
                  width={52}
                  height={52}
                  className="rounded-full shrink-0 border-2 border-white/20 object-cover"
                />
                <div>
                  <div className="text-xl font-bold text-white mb-1">
                    BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY
(DESHAPRIYA PARK)
                  </div>
                  <div className="text-sm text-gray-400">
                    Celebrating Tradition Since 1938 · Ballygunge, Kolkata
                  </div>
                </div>
              </div>

              {/* Quick links */}
              <nav className="flex flex-wrap items-center justify-center gap-5 text-sm">
                <a href="#activities" className="hover:text-orange-400 transition-colors">
                  Activities
                </a>
                <a href="#history" className="hover:text-orange-400 transition-colors">
                  History
                </a>
                <a href="#news" className="hover:text-orange-400 transition-colors">
                  News
                </a>
                <a href="#contact" className="hover:text-orange-400 transition-colors">
                  Contact
                </a>
                <Link href="/login" className="hover:text-orange-400 transition-colors">
                  Login
                </Link>
                <Link href="/membership-form" className="hover:text-orange-400 transition-colors">
                  Membership Form
                </Link>
              </nav>
            </div>

            <div className="mt-8 pt-6 border-t border-gray-800 text-center text-xs text-gray-500">
              &copy; {new Date().getFullYear()} BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY
(DESHAPRIYA PARK). All rights reserved.
            </div>
          </div>
        </footer>
      </main>
    </>
  );
}
