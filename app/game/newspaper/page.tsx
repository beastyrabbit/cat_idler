"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGameDashboard, formatDuration } from "@/hooks/useGameDashboard";
import { summarizeCatIdentity } from "@/lib/game/catTraits";

/* ================================================================
   THE CATFORD EXAMINER
   V9 - 1960s Broadsheet Newspaper UI Concept
   ================================================================ */

/* ───────────────────────────────────────────
   PALETTE & CONSTANTS
   ─────────────────────────────────────────── */
const INK = {
  paper: "#F5F0E8",
  paperDark: "#EDE6D6",
  ink: "#1A1A18",
  inkLight: "#3D3D38",
  inkFaded: "#8A8578",
  rule: "#B8B0A0",
  ruleHeavy: "#7A7468",
  red: "#8B1A1A",
  redBright: "#C0392B",
  green: "#2E5E1A",
  blue: "#1A3A5E",
  gold: "#8B6914",
  accent: "#D4A017",
  white: "#FEFCF5",
} as const;

const ORNAMENT = "\u2500\u2550\u2500\u2726\u2550\u2500";
const ORNAMENT_LONG = "\u2500\u2500\u2550\u2500\u2726\u2500\u2550\u2500\u2500";
const DIAMOND = "\u25C6";
const BULLET = "\u2022";

const SECTIONS = [
  { id: "section-headlines", label: "HEADLINES" },
  { id: "section-market", label: "MARKET" },
  { id: "section-community", label: "COMMUNITY" },
  { id: "section-news", label: "NEWS" },
  { id: "section-extras", label: "EXTRAS" },
] as const;

/* ───────────────────────────────────────────
   INJECTED KEYFRAMES & GLOBAL STYLES
   ─────────────────────────────────────────── */
const NEWSPAPER_STYLES = `
  @keyframes ce-fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }
  @keyframes ce-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
  @keyframes ce-flash {
    0% { background: ${INK.accent}22; }
    100% { background: transparent; }
  }

  .ce-page *::-webkit-scrollbar { width: 6px; }
  .ce-page *::-webkit-scrollbar-track { background: ${INK.paperDark}; }
  .ce-page *::-webkit-scrollbar-thumb { background: ${INK.rule}; border-radius: 3px; }

  .ce-btn {
    font-family: var(--font-playfair, Georgia, "Times New Roman", serif);
    font-weight: 700;
    border: 2px solid ${INK.ink};
    background: ${INK.paper};
    color: ${INK.ink};
    padding: 6px 16px;
    font-size: 12px;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition: background 150ms, color 150ms, transform 80ms;
    text-transform: uppercase;
  }
  .ce-btn:hover {
    background: ${INK.ink};
    color: ${INK.paper};
  }
  .ce-btn:active {
    transform: scale(0.97);
  }
  .ce-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .ce-btn:disabled:hover {
    background: ${INK.paper};
    color: ${INK.ink};
  }
  .ce-btn-red {
    border-color: ${INK.red};
    color: ${INK.red};
  }
  .ce-btn-red:hover {
    background: ${INK.red};
    color: ${INK.paper};
  }
  .ce-btn-red:disabled:hover {
    background: ${INK.paper};
    color: ${INK.red};
  }
  .ce-btn-green {
    border-color: ${INK.green};
    color: ${INK.green};
  }
  .ce-btn-green:hover {
    background: ${INK.green};
    color: ${INK.paper};
  }
  .ce-btn-green:disabled:hover {
    background: ${INK.paper};
    color: ${INK.green};
  }
  .ce-btn-blue {
    border-color: ${INK.blue};
    color: ${INK.blue};
  }
  .ce-btn-blue:hover {
    background: ${INK.blue};
    color: ${INK.paper};
  }

  .ce-dropcap::first-letter {
    float: left;
    font-family: var(--font-playfair, Georgia, "Times New Roman", serif);
    font-size: 3.2em;
    line-height: 0.85;
    padding-right: 6px;
    padding-top: 4px;
    color: ${INK.ink};
    font-weight: 700;
  }

  .ce-col-divider {
    width: 1px;
    background: ${INK.rule};
    flex-shrink: 0;
    align-self: stretch;
    margin: 0 0;
  }

  @keyframes ce-sectionReveal {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .ce-section-hidden {
    opacity: 0;
  }
  .ce-section-visible {
    animation: ce-sectionReveal 600ms ease-out forwards;
  }
  .ce-stagger-1 {
    animation-delay: 150ms;
  }

  /* ── Page turn: GPU-composited sweep overlay (no clip-path) ── */

  /* Paper overlay sweeps right→left to cover, then continues left to reveal */
  @keyframes ce-sweepCover {
    0%   { transform: translateX(105%) skewX(-5deg); }
    100% { transform: translateX(0%) skewX(-5deg); }
  }
  @keyframes ce-sweepReveal {
    0%   { transform: translateX(0%) skewX(-5deg); }
    100% { transform: translateX(-105%) skewX(-5deg); }
  }

  .ce-content-wrap {
    position: relative;
    overflow: hidden;
  }
  .ce-turning-out,
  .ce-turning-in {
    pointer-events: none;
  }

  /* The sweeping "page" overlay — sits inside content-wrap, hidden by default */
  .ce-content-wrap::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: -10%;
    width: 120%;
    z-index: 50;
    pointer-events: none;
    transform: translateX(105%) skewX(-5deg);
    will-change: transform;
    /* Fold shadow on leading edge, then solid paper */
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(0,0,0,0.10) 0.4%,
      rgba(0,0,0,0.06) 1%,
      ${INK.paperDark} 1.8%,
      ${INK.paper} 6%,
      ${INK.paper} 100%
    );
  }
  .ce-turning-out::before {
    animation: ce-sweepCover 380ms cubic-bezier(0.4, 0, 0.6, 1) forwards;
  }
  .ce-turning-in::before {
    animation: ce-sweepReveal 340ms cubic-bezier(0.3, 0, 0.5, 1) forwards;
  }

  /* ── Subscription modal ── */
  @keyframes ce-modalBackdrop {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  @keyframes ce-modalSlide {
    from { opacity: 0; transform: translateY(30px) scale(0.96); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }
  .ce-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(26, 26, 24, 0.55);
    backdrop-filter: blur(3px);
    animation: ce-modalBackdrop 400ms ease forwards;
  }
  .ce-modal-card {
    width: 440px;
    max-width: 92vw;
    background: ${INK.paper};
    border: 3px solid ${INK.ink};
    box-shadow: 8px 8px 0 ${INK.ink}44, 0 20px 60px rgba(0,0,0,0.3);
    padding: 0;
    animation: ce-modalSlide 500ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
    position: relative;
  }
  .ce-modal-input {
    width: 100%;
    background: ${INK.white};
    border: 2px solid ${INK.ink};
    color: ${INK.ink};
    padding: 10px 14px;
    font-family: var(--font-special-elite, 'Courier New', monospace);
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.04em;
    outline: none;
    transition: border-color 200ms;
    box-sizing: border-box;
  }
  .ce-modal-input:focus {
    border-color: ${INK.red};
  }
  .ce-modal-input::placeholder {
    color: ${INK.rule};
    font-weight: 400;
  }
`;

/* ───────────────────────────────────────────
   HELPERS
   ─────────────────────────────────────────── */

function jobLabel(kind: string): string {
  const labels: Record<string, string> = {
    supply_food: "Food Supply Run",
    supply_water: "Water Procurement",
    leader_plan_hunt: "Hunt Planning",
    leader_plan_house: "Construction Planning",
    hunt_expedition: "Hunting Expedition",
    build_house: "House Construction",
    ritual: "Sacred Ritual",
  };
  return labels[kind] ?? kind.replace(/_/g, " ");
}

function statusHeadline(
  status: string,
  food: number,
  water: number,
  maxFood: number,
  population: number,
): string {
  if (water < 5) return "CRISIS: WATER RESERVES DANGEROUSLY LOW";
  if (status === "dead") return "COLONY FALLS: ALL HOPE LOST";
  if (status === "struggling") {
    if (water < 20) return "CRISIS: WATER RESERVES DANGEROUSLY LOW";
    if (food < 20) return "FAMINE GRIPS COLONY AS FOOD STORES DWINDLE";
    return "COLONY STRUGGLES THROUGH DIFFICULT TIMES";
  }
  if (status === "thriving") {
    if (food > maxFood * 0.8)
      return "PROSPERITY! FOOD STORES SWELL TO RECORD HIGHS";
    if (population >= 8) return "COLONY THRIVES AS POPULATION REACHES NEW PEAK";
    return "GOLDEN AGE CONTINUES FOR HAPPY COLONY";
  }
  return "NEW COLONY ESTABLISHED: EARLY DAYS OF PROMISE";
}

function statusSubhead(status: string, population: number): string {
  if (status === "dead")
    return "Residents urged to remember the good times as colony disbands";
  if (status === "struggling")
    return `Emergency measures in effect across all ${population} souls`;
  if (status === "thriving")
    return `All ${population} residents report satisfaction with colony leadership`;
  return `${population} brave pioneers begin building a new life together`;
}

function moraleForecast(morale: number): {
  label: string;
  desc: string;
  icon: string;
} {
  if (morale > 80)
    return {
      label: "SUNNY",
      desc: "Clear skies and high spirits across the colony. Expect purring.",
      icon: "Bright & Clear",
    };
  if (morale > 60)
    return {
      label: "PARTLY CLOUDY",
      desc: "Generally pleasant conditions with occasional grumbling.",
      icon: "Fair",
    };
  if (morale > 40)
    return {
      label: "OVERCAST",
      desc: "A pall of discontent hangs over the colony. Naps are restless.",
      icon: "Unsettled",
    };
  if (morale > 20)
    return {
      label: "STORMY",
      desc: "Tempers flare and hissing is commonplace. Take cover.",
      icon: "Rough",
    };
  return {
    label: "CATASTROPHIC",
    desc: "Colony morale at rock bottom. Even the mice feel sorry for us.",
    icon: "Dire",
  };
}

function resourcePct(val: number, max: number): number {
  return max > 0 ? Math.min(100, Math.round((val / max) * 100)) : 0;
}

function editionNumber(runNumber: number): string {
  const ed = runNumber * 147 + 3892;
  return ed.toLocaleString();
}

function formatDate(ts: number): string {
  return new Date(ts).toLocaleDateString("en-GB", {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

/* ───────────────────────────────────────────
   DECORATIVE SUBCOMPONENTS
   ─────────────────────────────────────────── */

function HeavyRule() {
  return (
    <div
      style={{
        borderTop: `3px double ${INK.ink}`,
        margin: "2px 0",
      }}
    />
  );
}

function LightRule() {
  return (
    <div
      style={{
        borderTop: `1px solid ${INK.rule}`,
        margin: "6px 0",
      }}
    />
  );
}

function OrnamentalRule() {
  return (
    <div
      style={{
        textAlign: "center",
        fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
        fontSize: 14,
        color: INK.ruleHeavy,
        letterSpacing: "0.3em",
        margin: "6px 0",
        userSelect: "none",
      }}
    >
      {ORNAMENT_LONG}
    </div>
  );
}

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 8 }}>
      <div
        style={{
          borderTop: `2px solid ${INK.ink}`,
          borderBottom: `1px solid ${INK.ink}`,
          padding: "3px 0",
          textAlign: "center",
          fontFamily: "var(--font-playfair, Georgia, 'Times New Roman', serif)",
          fontSize: 13,
          fontWeight: 700,
          letterSpacing: "0.2em",
          textTransform: "uppercase",
          color: INK.ink,
        }}
      >
        {children}
      </div>
    </div>
  );
}

/* ───────────────────────────────────────────
   SUBSCRIPTION STORAGE
   ─────────────────────────────────────────── */

const STORAGE_KEY = "catford-examiner-subscriber";

type SubscriberData = {
  name: string;
  subscribedAt: number;
  ipHash: string;
};

function getStoredSubscriber(): SubscriberData | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof (parsed as SubscriberData).name === "string" &&
      (parsed as SubscriberData).name.length > 0
    ) {
      return parsed as SubscriberData;
    }
    console.warn("catford-examiner: invalid subscriber data, ignoring");
    return null;
  } catch (err) {
    console.warn("catford-examiner: failed to read subscriber data", err);
    return null;
  }
}

function storeSubscriber(data: SubscriberData): boolean {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    return true;
  } catch (err) {
    console.warn("catford-examiner: failed to persist subscriber data", err);
    return false;
  }
}

async function fetchIpHash(): Promise<string> {
  try {
    const res = await fetch("/api/subscriber-hash");
    if (!res.ok) {
      console.warn(`subscriber-hash: HTTP ${res.status}`);
      return "unknown";
    }
    const data = await res.json();
    return data.hash ?? "unknown";
  } catch (err) {
    console.warn("subscriber-hash: fetch failed", err);
    return "unknown";
  }
}

/* ───────────────────────────────────────────
   SUBSCRIPTION MODAL
   ─────────────────────────────────────────── */

function SubscriptionModal({
  onSubscribe,
}: {
  onSubscribe: (name: string) => void;
}) {
  const [name, setName] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Delay focus until ce-modalSlide animation (500ms) completes
    const timer = setTimeout(() => inputRef.current?.focus(), 500);
    return () => clearTimeout(timer);
  }, []);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed) onSubscribe(trimmed);
  };

  return (
    <div className="ce-modal-backdrop">
      <div className="ce-modal-card">
        {/* Top ornamental rule */}
        <div style={{ borderBottom: `4px solid ${INK.ink}`, padding: "2px 0" }}>
          <div style={{ borderBottom: `2px solid ${INK.ink}` }} />
        </div>

        {/* Header */}
        <div style={{ padding: "16px 24px 0", textAlign: "center" }}>
          <div
            style={{
              fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
              fontSize: 10,
              letterSpacing: "0.2em",
              textTransform: "uppercase",
              color: INK.inkFaded,
              marginBottom: 4,
            }}
          >
            Since 1962 {DIAMOND} Trusted by Cats Everywhere
          </div>
          <div
            style={{
              fontFamily:
                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
              fontSize: "clamp(28px, 5vw, 40px)",
              fontWeight: 900,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              lineHeight: 1,
              color: INK.ink,
            }}
          >
            The Catford Examiner
          </div>
          <div
            style={{
              fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
              fontSize: 10,
              letterSpacing: "0.2em",
              color: INK.inkFaded,
              marginTop: 4,
              textTransform: "uppercase",
            }}
          >
            &ldquo;All the Mews That&apos;s Fit to Print&rdquo;
          </div>
        </div>

        {/* Ornamental separator */}
        <div
          style={{
            textAlign: "center",
            fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
            fontSize: 14,
            color: INK.ruleHeavy,
            letterSpacing: "0.3em",
            margin: "10px 0 6px",
            userSelect: "none",
          }}
        >
          {ORNAMENT_LONG}
        </div>

        {/* Body */}
        <div style={{ padding: "0 24px 20px" }}>
          <div
            style={{
              fontFamily:
                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
              fontSize: 18,
              fontWeight: 900,
              textTransform: "uppercase",
              textAlign: "center",
              letterSpacing: "0.08em",
              lineHeight: 1.2,
              marginBottom: 6,
            }}
          >
            Subscribe Today!
          </div>
          <div
            style={{
              fontFamily:
                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
              fontSize: 12,
              fontStyle: "italic",
              color: INK.inkLight,
              textAlign: "center",
              lineHeight: 1.4,
              marginBottom: 16,
            }}
          >
            Receive the latest colony dispatches, market reports, and society
            gossip delivered directly to your screen. Subscription is free (we
            are funded entirely by kibble donations).
          </div>

          <form onSubmit={handleSubmit}>
            <div
              style={{
                fontFamily:
                  "var(--font-special-elite, 'Courier New', monospace)",
                fontSize: 10,
                letterSpacing: "0.15em",
                textTransform: "uppercase",
                color: INK.inkFaded,
                marginBottom: 4,
              }}
            >
              Your Name, Dear Reader:
            </div>
            <input
              ref={inputRef}
              className="ce-modal-input"
              type="text"
              placeholder="e.g. Lord Whiskersworth III"
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={30}
              autoComplete="off"
            />

            <button
              type="submit"
              className="ce-btn"
              disabled={!name.trim()}
              style={{
                width: "100%",
                marginTop: 12,
                padding: "10px 16px",
                fontSize: 14,
                letterSpacing: "0.1em",
              }}
            >
              Begin My Subscription
            </button>
          </form>

          <div
            style={{
              fontSize: 9,
              color: INK.rule,
              textAlign: "center",
              marginTop: 10,
              fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
              fontStyle: "italic",
            }}
          >
            By subscribing you agree to receive an unreasonable number of cat
            puns. Unsubscribe by closing your browser and pretending this never
            happened.
          </div>
        </div>

        {/* Bottom ornamental rule */}
        <div style={{ borderTop: `2px solid ${INK.ink}`, padding: "2px 0" }}>
          <div style={{ borderTop: `4px solid ${INK.ink}` }} />
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════
   MAIN PAGE COMPONENT
   ═══════════════════════════════════════════ */
export default function CatfordExaminerPage() {
  const {
    dashboard,
    colony,
    jobs,
    upgrades,
    events,
    cats,
    ritualPoints,
    accelerationPreset,
    statusTone,
    now,
    sessionId,
    nickname,
    showTestControls,
    busyAction,
    error,
    submitJob,
    onBoostJob,
    onBuyUpgrade,
    onSetAcceleration,
    onAdvanceTime,
    updateNickname,
    leader,
    onlineCount,
  } = useGameDashboard();

  const [expandedCatId, setExpandedCatId] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<string>(SECTIONS[0].id);
  const [isNavStuck, setIsNavStuck] = useState(false);
  const [revealedSections, setRevealedSections] = useState<Set<string>>(
    () => new Set(),
  );
  const [pageTurnPhase, setPageTurnPhase] = useState<"idle" | "out" | "in">(
    "idle",
  );
  const [showSubscribeModal, setShowSubscribeModal] = useState(false);
  const navSentinelRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const pendingScrollRef = useRef<string | null>(null);
  const pageTurnTimersRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  // Check localStorage for existing subscription on mount.
  // Intentionally runs once — nickname/updateNickname are stable after first render.
  useEffect(() => {
    const existing = getStoredSubscriber();
    if (existing) {
      updateNickname(existing.name);
    } else {
      setShowSubscribeModal(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSubscribe = useCallback(
    async (subscriberName: string) => {
      const ipHash = await fetchIpHash();
      storeSubscriber({
        name: subscriberName,
        subscribedAt: Date.now(),
        ipHash,
      });
      updateNickname(subscriberName);
      setShowSubscribeModal(false);
    },
    [updateNickname],
  );

  const hasData = !!dashboard;

  // Derived data
  const res = useMemo(() => {
    if (!colony)
      return { food: 0, water: 0, herbs: 0, materials: 0, blessings: 0 };
    const r = colony.resources as Record<string, number>;
    return {
      food: Math.floor(r.food ?? 0),
      water: Math.floor(r.water ?? 0),
      herbs: Math.floor(r.herbs ?? 0),
      materials: Math.floor(r.materials ?? 0),
      blessings: Math.floor(r.blessings ?? 0),
    };
  }, [colony]);

  const maxFood = 200;
  const maxWater = 200;
  const maxMaterials = 200;

  const aliveCats = useMemo(
    () => cats.filter((c: any) => !c.deathTime),
    [cats],
  );
  const deadCats = useMemo(() => cats.filter((c: any) => c.deathTime), [cats]);

  const activeJobs = useMemo(
    () => jobs.filter((j: any) => j.status === "active"),
    [jobs],
  );
  const queuedJobs = useMemo(
    () => jobs.filter((j: any) => j.status === "queued"),
    [jobs],
  );

  const morale = useMemo(() => {
    if (!colony) return 50;
    // Estimate morale from food and water supply levels
    const r = colony.resources as Record<string, number>;
    const foodPct = Math.min(100, ((r.food ?? 0) / maxFood) * 100);
    const waterPct = Math.min(100, ((r.water ?? 0) / maxWater) * 100);
    return Math.round((foodPct + waterPct) / 2);
  }, [colony]);

  const forecast = moraleForecast(morale);

  const scrollToSection = useCallback(
    (id: string) => {
      if (pageTurnPhase !== "idle") return;
      const el = document.getElementById(id);
      if (!el) return;

      // If we're already at this section, do nothing
      if (activeSection === id) return;

      // Clear any in-flight page turn timers
      pageTurnTimersRef.current.forEach(clearTimeout);
      pageTurnTimersRef.current = [];

      pendingScrollRef.current = id;
      setPageTurnPhase("out");

      // After the sweep-cover animation, scroll instantly and sweep-reveal
      const t1 = setTimeout(() => {
        el.scrollIntoView({ behavior: "instant", block: "start" });
        setActiveSection(id);
        setPageTurnPhase("in");

        // After the sweep-reveal animation, return to idle
        const t2 = setTimeout(() => {
          setPageTurnPhase("idle");
          pendingScrollRef.current = null;
        }, 360);
        pageTurnTimersRef.current.push(t2);
      }, 400);
      pageTurnTimersRef.current.push(t1);
    },
    [pageTurnPhase, activeSection],
  );

  // Clean up page-turn timers on unmount
  useEffect(() => {
    return () => {
      pageTurnTimersRef.current.forEach(clearTimeout);
    };
  }, []);

  // Track which section is in view (updates active nav item)
  useEffect(() => {
    const root = scrollContainerRef.current;
    if (!root) return;
    const ids = SECTIONS.map((s) => s.id);
    const observer = new IntersectionObserver(
      (entries) => {
        // Find the topmost visible section
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (visible.length > 0) {
          setActiveSection(visible[0].target.id);
        }
      },
      { root, threshold: 0.15 },
    );
    ids.forEach((id) => {
      const el = document.getElementById(id);
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, [hasData]); // re-attach once after data loads

  // Detect when nav becomes sticky
  useEffect(() => {
    const sentinel = navSentinelRef.current;
    const root = scrollContainerRef.current;
    if (!sentinel || !root) return;
    const observer = new IntersectionObserver(
      ([entry]) => setIsNavStuck(!entry.isIntersecting),
      { root, threshold: 0 },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [hasData]);

  // Reveal sections once on first scroll into view
  useEffect(() => {
    const root = scrollContainerRef.current;
    if (!root) return;
    const ids = SECTIONS.map((s) => s.id);
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setRevealedSections((prev) => {
              if (prev.has(entry.target.id)) return prev;
              const next = new Set(prev);
              next.add(entry.target.id);
              return next;
            });
            observer.unobserve(entry.target);
          }
        });
      },
      { root, threshold: 0.1 },
    );
    ids.forEach((id) => {
      const el = document.getElementById(id);
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, [hasData]);

  /* ─── LOADING STATE ─── */
  if (dashboard === undefined) {
    return (
      <div
        className="ce-page"
        style={{
          position: "fixed",
          inset: 0,
          background: INK.paper,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: "var(--font-playfair, Georgia, 'Times New Roman', serif)",
          zIndex: 50,
        }}
      >
        <style>{NEWSPAPER_STYLES}</style>
        <div
          style={{
            fontSize: 28,
            fontWeight: 700,
            letterSpacing: "0.15em",
            textTransform: "uppercase",
            color: INK.ink,
            marginBottom: 16,
          }}
        >
          The Catford Examiner
        </div>
        <div
          style={{
            fontSize: 14,
            color: INK.inkFaded,
            fontStyle: "italic",
            animation: "ce-pulse 2s ease-in-out infinite",
          }}
        >
          Preparing Global Colony...
        </div>
        <div
          style={{
            marginTop: 24,
            fontSize: 12,
            color: INK.rule,
            letterSpacing: "0.3em",
          }}
        >
          {ORNAMENT_LONG}
        </div>
      </div>
    );
  }

  if (!dashboard || !colony) {
    return (
      <div
        className="ce-page"
        style={{
          position: "fixed",
          inset: 0,
          background: INK.paper,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: "var(--font-playfair, Georgia, 'Times New Roman', serif)",
          zIndex: 50,
        }}
      >
        <style>{NEWSPAPER_STYLES}</style>
        <div
          style={{
            fontSize: 36,
            fontWeight: 700,
            color: INK.ink,
            marginBottom: 12,
          }}
        >
          EXTRA! EXTRA!
        </div>
        <div
          style={{
            fontSize: 16,
            color: INK.inkLight,
            fontStyle: "italic",
            marginBottom: 24,
          }}
        >
          No colony found. The presses stand idle.
        </div>
      </div>
    );
  }

  const population = aliveCats.length;
  const headline = statusHeadline(
    colony.status,
    res.food,
    res.water,
    maxFood,
    population,
  );
  const subhead = statusSubhead(colony.status, population);

  /* ═══════════════════════════════════════════
     RENDER
     ═══════════════════════════════════════════ */
  return (
    <div
      ref={scrollContainerRef}
      className="ce-page"
      style={{
        position: "fixed",
        inset: 0,
        background: INK.paper,
        color: INK.ink,
        overflow: "auto",
        zIndex: 50,
        fontFamily: "var(--font-inter, Georgia, 'Times New Roman', serif)",
        fontSize: 13,
        lineHeight: 1.5,
        /* Subtle paper texture via noise */
        backgroundImage: `
          url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='0.03'/%3E%3C/svg%3E")
        `,
        backgroundSize: "256px 256px",
      }}
    >
      <style>{NEWSPAPER_STYLES}</style>

      {/* Subscription modal — shown once on first visit */}
      {showSubscribeModal && (
        <SubscriptionModal onSubscribe={handleSubscribe} />
      )}

      {/* Outer container for max-width broadsheet feel */}
      <div
        style={{
          maxWidth: 1400,
          margin: "0 auto",
          padding: "0 20px 40px",
        }}
      >
        {/* ═══════════════════════════════════════
           MASTHEAD
           ═══════════════════════════════════════ */}
        <header style={{ paddingTop: 16 }}>
          {/* Top info line */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              fontSize: 10,
              fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
              color: INK.inkFaded,
              letterSpacing: "0.1em",
              textTransform: "uppercase",
              marginBottom: 4,
            }}
          >
            <span>{formatDate(now)}</span>
            <span style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <span>
                {onlineCount} Reader{onlineCount !== 1 ? "s" : ""} Online
              </span>
              <span>{DIAMOND}</span>
              <span>Run #{colony.runNumber ?? 1}</span>
            </span>
            <span>Price: 2 Kibble</span>
          </div>

          {/* Heavy rule above masthead */}
          <div
            style={{
              borderTop: `4px solid ${INK.ink}`,
              borderBottom: `2px solid ${INK.ink}`,
              paddingTop: 1,
              marginBottom: 1,
            }}
          />

          {/* Newspaper title */}
          <div
            style={{
              textAlign: "center",
              padding: "8px 0 4px",
            }}
          >
            <div
              style={{
                fontFamily:
                  "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                fontSize: "clamp(36px, 6vw, 72px)",
                fontWeight: 900,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                lineHeight: 1,
                color: INK.ink,
              }}
            >
              The Catford Examiner
            </div>
            <div
              style={{
                fontFamily:
                  "var(--font-special-elite, 'Courier New', monospace)",
                fontSize: 11,
                letterSpacing: "0.25em",
                color: INK.inkFaded,
                marginTop: 4,
                textTransform: "uppercase",
              }}
            >
              {colony.name} {DIAMOND} Est. Edition No.{" "}
              {editionNumber(colony.runNumber ?? 1)} {DIAMOND} &ldquo;All the
              Mews That&apos;s Fit to Print&rdquo;
            </div>
          </div>

          {/* Heavy rule below masthead */}
          <div
            style={{
              borderTop: `2px solid ${INK.ink}`,
              borderBottom: `4px solid ${INK.ink}`,
              paddingTop: 1,
            }}
          />
        </header>

        {/* ═══════════════════════════════════════
           ERROR BAR
           ═══════════════════════════════════════ */}
        {error && (
          <div
            style={{
              background: INK.red,
              color: INK.white,
              padding: "8px 16px",
              marginTop: 8,
              fontFamily:
                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
              fontWeight: 700,
              fontSize: 13,
              textAlign: "center",
              letterSpacing: "0.05em",
            }}
          >
            CORRECTION NOTICE: {error}
          </div>
        )}

        {/* ═══════════════════════════════════════
           TEST CONTROLS
           ═══════════════════════════════════════ */}
        {showTestControls && (
          <div
            style={{
              border: `2px solid ${INK.gold}`,
              background: `${INK.accent}15`,
              padding: "8px 16px",
              marginTop: 8,
              display: "flex",
              alignItems: "center",
              gap: 12,
              fontSize: 11,
              fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
            }}
          >
            <span
              style={{
                fontWeight: 700,
                letterSpacing: "0.1em",
                textTransform: "uppercase",
              }}
            >
              Press Room Controls // Time Warp:{" "}
              {accelerationPreset.toUpperCase()}
            </span>
            <button
              className="ce-btn"
              onClick={() => void onSetAcceleration("off")}
              disabled={busyAction === "accel:off"}
              style={{ fontSize: 10, padding: "3px 10px" }}
            >
              Off
            </button>
            <button
              className="ce-btn"
              onClick={() => void onSetAcceleration("fast")}
              disabled={busyAction === "accel:fast"}
              style={{ fontSize: 10, padding: "3px 10px" }}
            >
              Fast
            </button>
            <button
              className="ce-btn ce-btn-red"
              onClick={() => void onSetAcceleration("turbo")}
              disabled={busyAction === "accel:turbo"}
              style={{ fontSize: 10, padding: "3px 10px" }}
            >
              Turbo
            </button>
            <button
              className="ce-btn ce-btn-green"
              onClick={() => void onAdvanceTime(300)}
              disabled={busyAction === "advance:300"}
              style={{ fontSize: 10, padding: "3px 10px" }}
            >
              Skip +5m
            </button>
            <button
              className="ce-btn ce-btn-green"
              onClick={() => void onAdvanceTime(3600)}
              disabled={busyAction === "advance:3600"}
              style={{ fontSize: 10, padding: "3px 10px" }}
            >
              Skip +1h
            </button>
          </div>
        )}

        {/* ═══════════════════════════════════════
           SECTION: HEADLINES (Masthead Lead)
           ═══════════════════════════════════════ */}
        <section
          id="section-headlines"
          className={
            revealedSections.has("section-headlines")
              ? "ce-section-visible"
              : "ce-section-hidden"
          }
          style={{ scrollMarginTop: 48, marginTop: 12, marginBottom: 4 }}
        >
          <h1
            style={{
              fontFamily:
                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
              fontSize: "clamp(28px, 4vw, 48px)",
              fontWeight: 900,
              lineHeight: 1.05,
              letterSpacing: "-0.01em",
              textTransform: "uppercase",
              margin: 0,
              color:
                colony.status === "dead" || colony.status === "struggling"
                  ? INK.red
                  : INK.ink,
            }}
          >
            {headline}
          </h1>
          <div
            style={{
              fontFamily:
                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
              fontSize: 16,
              fontStyle: "italic",
              color: INK.inkLight,
              marginTop: 4,
              lineHeight: 1.3,
            }}
          >
            {subhead}
          </div>
        </section>

        {/* ─── Nav sentinel (zero-height div for sticky detection) ─── */}
        <div ref={navSentinelRef} style={{ height: 0, margin: 0 }} />

        {/* ═══════════════════════════════════════
           NAVIGATION BAR
           ═══════════════════════════════════════ */}
        <nav
          style={{
            position: "sticky",
            top: 0,
            zIndex: 10,
            background: INK.paper,
            borderTop: `2px solid ${INK.ink}`,
            borderBottom: `2px solid ${INK.ink}`,
            padding: "6px 0",
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            gap: 4,
            boxShadow: isNavStuck ? "0 1px 3px rgba(0,0,0,0.08)" : "none",
            transition: "box-shadow 200ms ease",
          }}
        >
          {SECTIONS.map((s, i) => (
            <span
              key={s.id}
              style={{ display: "flex", alignItems: "center", gap: 4 }}
            >
              {i > 0 && (
                <span
                  style={{
                    color: INK.rule,
                    fontSize: 8,
                    userSelect: "none",
                  }}
                >
                  {DIAMOND}
                </span>
              )}
              <button
                onClick={() => scrollToSection(s.id)}
                style={{
                  background: "none",
                  border: "none",
                  borderBottom:
                    activeSection === s.id
                      ? `2px solid ${INK.red}`
                      : "2px solid transparent",
                  fontFamily:
                    "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                  fontSize: 11,
                  fontWeight: 700,
                  letterSpacing: "0.2em",
                  textTransform: "uppercase",
                  color: activeSection === s.id ? INK.red : INK.inkFaded,
                  cursor: "pointer",
                  padding: "2px 6px",
                  transition: "color 150ms, border-color 150ms",
                }}
                onMouseEnter={(e) => {
                  if (activeSection !== s.id)
                    (e.target as HTMLElement).style.color = INK.ink;
                }}
                onMouseLeave={(e) => {
                  if (activeSection !== s.id)
                    (e.target as HTMLElement).style.color = INK.inkFaded;
                }}
              >
                {s.label}
              </button>
            </span>
          ))}
        </nav>

        <HeavyRule />

        {/* Page-turn content wrapper — ::before pseudo sweeps across */}
        <div
          className={`ce-content-wrap${
            { out: " ce-turning-out", in: " ce-turning-in", idle: "" }[
              pageTurnPhase
            ]
          }`}
        >
          {/* ═══════════════════════════════════════
           SECTION: MARKET
           ═══════════════════════════════════════ */}
          <section
            id="section-market"
            className={
              revealedSections.has("section-market")
                ? "ce-section-visible"
                : "ce-section-hidden"
            }
            style={{ scrollMarginTop: 48, marginTop: 8 }}
          >
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1.8fr 1px 1fr",
                gap: 0,
              }}
            >
              {/* Left: Leader Lede + Market Report + Ritual Points */}
              <div style={{ padding: "0 16px 0 0" }}>
                {/* Leader lede */}
                {leader && (
                  <div style={{ marginBottom: 12 }}>
                    <div
                      style={{
                        fontFamily:
                          "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                        fontSize: 11,
                        fontWeight: 700,
                        letterSpacing: "0.15em",
                        textTransform: "uppercase",
                        color: INK.inkFaded,
                        marginBottom: 4,
                      }}
                    >
                      From the Office of the Colony Leader
                    </div>
                    <div
                      className="ce-dropcap"
                      style={{ textAlign: "justify" }}
                    >
                      {(leader as any).name} continues to serve as Leader of{" "}
                      {colony.name}, guiding our colony of {population} souls
                      through{" "}
                      {(
                        {
                          thriving: "a period of unprecedented prosperity",
                          struggling:
                            "these difficult times with unwavering determination",
                        } as Record<string, string>
                      )[colony.status] ??
                        "the early stages of settlement with cautious optimism"}
                      . The Leader reports{" "}
                      {activeJobs.length > 0
                        ? `${activeJobs.length} active operations currently underway`
                        : "no active operations at this time"}
                      {queuedJobs.length > 0
                        ? ` with ${queuedJobs.length} more in planning stages`
                        : ""}
                      . Citizens are encouraged to contribute to supply efforts
                      via the usual channels.{" "}
                      {ritualPoints > 0
                        ? `The colony treasury holds ${Math.floor(ritualPoints)} ritual points for improvements.`
                        : "The ritual treasury stands empty."}
                    </div>
                  </div>
                )}

                {!leader && (
                  <div style={{ marginBottom: 12 }}>
                    <div
                      className="ce-dropcap"
                      style={{ textAlign: "justify" }}
                    >
                      The colony of {colony.name} proceeds without formal
                      leadership as {population} residents organize themselves
                      into a functioning community. Residents report{" "}
                      {colony.status === "thriving"
                        ? "general satisfaction"
                        : "growing concern"}{" "}
                      with present conditions.{" "}
                      {ritualPoints > 0
                        ? `The community treasury holds ${Math.floor(ritualPoints)} ritual points.`
                        : ""}
                    </div>
                  </div>
                )}

                <OrnamentalRule />

                {/* ─── MARKET REPORT (Resources) ─── */}
                <SectionHeader>Market Report</SectionHeader>

                <div
                  style={{
                    fontFamily:
                      "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                    fontSize: 11,
                    fontStyle: "italic",
                    color: INK.inkFaded,
                    marginBottom: 8,
                  }}
                >
                  Colony Resource Exchange {DIAMOND} Closing Figures
                </div>

                {/* Resource bars styled as market tickers */}
                {[
                  {
                    label: "FOOD",
                    value: res.food,
                    max: maxFood,
                    trend:
                      res.food > maxFood * 0.6
                        ? "STRONG"
                        : res.food > maxFood * 0.3
                          ? "STEADY"
                          : "WEAK",
                    color: res.food < maxFood * 0.25 ? INK.red : INK.green,
                  },
                  {
                    label: "WATER",
                    value: res.water,
                    max: maxWater,
                    trend:
                      res.water > maxWater * 0.6
                        ? "STRONG"
                        : res.water > maxWater * 0.3
                          ? "STEADY"
                          : "WEAK",
                    color: res.water < maxWater * 0.25 ? INK.red : INK.blue,
                  },
                  {
                    label: "HERBS",
                    value: res.herbs,
                    max: 200,
                    trend: res.herbs > 50 ? "ADEQUATE" : "LOW",
                    color: res.herbs < 25 ? INK.red : INK.green,
                  },
                  {
                    label: "MATERIALS",
                    value: res.materials,
                    max: maxMaterials,
                    trend:
                      res.materials > 100
                        ? "ABUNDANT"
                        : res.materials > 40
                          ? "SUFFICIENT"
                          : "SCARCE",
                    color:
                      res.materials < maxMaterials * 0.2
                        ? INK.red
                        : INK.inkLight,
                  },
                ].map(({ label, value, max, trend, color }) => {
                  const pct = resourcePct(value, max);
                  return (
                    <div key={label} style={{ marginBottom: 10 }}>
                      <div
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          alignItems: "baseline",
                          marginBottom: 2,
                        }}
                      >
                        <span
                          style={{
                            fontFamily:
                              "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                            fontWeight: 700,
                            fontSize: 12,
                            letterSpacing: "0.1em",
                          }}
                        >
                          {label}
                        </span>
                        <span
                          style={{
                            fontFamily:
                              "var(--font-special-elite, 'Courier New', monospace)",
                            fontSize: 11,
                            color: color,
                            fontWeight: 700,
                          }}
                        >
                          {value}/{max} {BULLET} {trend}
                        </span>
                      </div>
                      {/* Bar chart - old school */}
                      <div
                        style={{
                          height: 12,
                          background: `${INK.rule}40`,
                          border: `1px solid ${INK.rule}`,
                          position: "relative",
                          overflow: "hidden",
                        }}
                      >
                        <div
                          style={{
                            position: "absolute",
                            top: 0,
                            left: 0,
                            height: "100%",
                            width: `${pct}%`,
                            background: color,
                            opacity: 0.7,
                            transition: "width 600ms ease",
                          }}
                        />
                        {/* Hatching lines for vintage feel */}
                        <div
                          style={{
                            position: "absolute",
                            top: 0,
                            left: 0,
                            height: "100%",
                            width: `${pct}%`,
                            background: `repeating-linear-gradient(
                            45deg,
                            transparent,
                            transparent 2px,
                            ${INK.paper}30 2px,
                            ${INK.paper}30 4px
                          )`,
                          }}
                        />
                      </div>
                    </div>
                  );
                })}

                {/* Ritual Points callout */}
                <div
                  style={{
                    marginTop: 8,
                    padding: "8px 12px",
                    border: `2px solid ${INK.gold}`,
                    background: `${INK.accent}08`,
                    textAlign: "center",
                  }}
                >
                  <div
                    style={{
                      fontFamily:
                        "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                      fontSize: 10,
                      letterSpacing: "0.2em",
                      textTransform: "uppercase",
                      color: INK.gold,
                      fontWeight: 700,
                      marginBottom: 2,
                    }}
                  >
                    Colony Treasury
                  </div>
                  <div
                    style={{
                      fontFamily:
                        "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                      fontSize: 28,
                      fontWeight: 900,
                      color: INK.gold,
                      lineHeight: 1,
                    }}
                  >
                    {Math.floor(ritualPoints)}
                  </div>
                  <div
                    style={{
                      fontSize: 10,
                      color: INK.inkFaded,
                      fontStyle: "italic",
                    }}
                  >
                    Ritual Points Available
                  </div>
                </div>
              </div>

              {/* Column divider */}
              <div className="ce-col-divider" />

              {/* Right: Situations Wanted (Jobs) */}
              <div style={{ padding: "0 0 0 14px" }} className="ce-stagger-1">
                <SectionHeader>Situations Wanted</SectionHeader>

                {jobs.length === 0 ? (
                  <div
                    style={{
                      textAlign: "center",
                      fontStyle: "italic",
                      color: INK.inkFaded,
                      padding: "12px 0",
                      fontSize: 12,
                    }}
                  >
                    No positions currently filled. All cats at leisure.
                  </div>
                ) : (
                  <div>
                    {jobs.map((job: any) => {
                      const totalMs = Math.max(1, job.endsAt - job.startedAt);
                      const doneMs = Math.max(
                        0,
                        Math.min(totalMs, now - job.startedAt),
                      );
                      const pct = Math.floor((doneMs / totalMs) * 100);
                      const remaining = formatDuration(job.endsAt - now);
                      const isActive = job.status === "active";

                      return (
                        <div
                          key={job._id}
                          style={{
                            padding: "6px 0",
                            borderBottom: `1px dotted ${INK.rule}`,
                          }}
                        >
                          {/* Job title */}
                          <div
                            style={{
                              fontFamily:
                                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                              fontWeight: 700,
                              fontSize: 13,
                              lineHeight: 1.2,
                              textTransform: "uppercase",
                              letterSpacing: "0.03em",
                            }}
                          >
                            {jobLabel(job.kind)}
                          </div>
                          {/* Details line */}
                          <div
                            style={{
                              fontSize: 10,
                              color: INK.inkFaded,
                              fontFamily:
                                "var(--font-special-elite, 'Courier New', monospace)",
                              display: "flex",
                              justifyContent: "space-between",
                              marginTop: 2,
                            }}
                          >
                            <span>
                              {job.assignedCatName
                                ? `Assigned: ${job.assignedCatName}`
                                : job.status === "queued"
                                  ? "Awaiting assignment"
                                  : "Unassigned"}
                            </span>
                            <span
                              style={{
                                color: isActive ? INK.inkLight : INK.inkFaded,
                                fontWeight: isActive ? 700 : 400,
                              }}
                            >
                              {job.status.toUpperCase()}
                            </span>
                          </div>
                          {/* Progress bar */}
                          <div
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: 6,
                              marginTop: 4,
                            }}
                          >
                            <div
                              style={{
                                flex: 1,
                                height: 8,
                                background: `${INK.rule}30`,
                                border: `1px solid ${INK.rule}`,
                                position: "relative",
                                overflow: "hidden",
                              }}
                            >
                              <div
                                style={{
                                  position: "absolute",
                                  top: 0,
                                  left: 0,
                                  height: "100%",
                                  width: `${pct}%`,
                                  background:
                                    pct > 75
                                      ? INK.green
                                      : pct > 40
                                        ? INK.gold
                                        : INK.inkLight,
                                  transition: "width 500ms linear",
                                }}
                              />
                            </div>
                            <span
                              style={{
                                fontFamily:
                                  "var(--font-special-elite, 'Courier New', monospace)",
                                fontSize: 10,
                                fontWeight: 700,
                                minWidth: 32,
                                textAlign: "right",
                              }}
                            >
                              {pct}%
                            </span>
                          </div>
                          {/* ETA + Boost */}
                          <div
                            style={{
                              display: "flex",
                              justifyContent: "space-between",
                              alignItems: "center",
                              marginTop: 3,
                            }}
                          >
                            <span
                              style={{
                                fontSize: 10,
                                fontFamily:
                                  "var(--font-special-elite, 'Courier New', monospace)",
                                color: INK.inkFaded,
                              }}
                            >
                              ETA: {remaining}
                              {job.totalClicks > 0
                                ? ` ${BULLET} ${job.totalClicks} boost${job.totalClicks !== 1 ? "s" : ""}`
                                : ""}
                            </span>
                            {isActive && (
                              <button
                                className="ce-btn"
                                onClick={() => void onBoostJob(job._id)}
                                disabled={busyAction === job._id}
                                style={{
                                  fontSize: 9,
                                  padding: "2px 8px",
                                  borderWidth: 1,
                                }}
                              >
                                Boost
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          </section>

          <HeavyRule />

          {/* ═══════════════════════════════════════
           SECTION: COMMUNITY
           ═══════════════════════════════════════ */}
          <section
            id="section-community"
            className={
              revealedSections.has("section-community")
                ? "ce-section-visible"
                : "ce-section-hidden"
            }
            style={{ scrollMarginTop: 48, marginTop: 8 }}
          >
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1px 1.4fr",
                gap: 0,
              }}
            >
              {/* Left: Place Your Order + Nickname */}
              <div style={{ padding: "0 16px 0 0" }}>
                <SectionHeader>Place Your Order</SectionHeader>
                <div
                  style={{
                    fontFamily:
                      "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                    fontSize: 11,
                    fontStyle: "italic",
                    color: INK.inkFaded,
                    marginBottom: 8,
                  }}
                >
                  Citizens may submit the following requisitions
                </div>

                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 6,
                  }}
                >
                  <button
                    className="ce-btn ce-btn-green"
                    onClick={() => void submitJob("supply_food")}
                    disabled={busyAction === "supply_food"}
                    style={{ fontSize: 11, width: "100%" }}
                  >
                    Supply Food (20s)
                  </button>
                  <button
                    className="ce-btn ce-btn-blue"
                    onClick={() => void submitJob("supply_water")}
                    disabled={busyAction === "supply_water"}
                    style={{ fontSize: 11, width: "100%" }}
                  >
                    Supply Water (15s)
                  </button>
                  <button
                    className="ce-btn"
                    onClick={() => void submitJob("leader_plan_hunt")}
                    disabled={busyAction === "leader_plan_hunt"}
                    style={{ fontSize: 11, width: "100%" }}
                  >
                    Request Hunt
                  </button>
                  <button
                    className="ce-btn"
                    onClick={() => void submitJob("leader_plan_house")}
                    disabled={busyAction === "leader_plan_house"}
                    style={{ fontSize: 11, width: "100%" }}
                  >
                    Request Build
                  </button>
                  <button
                    className="ce-btn ce-btn-red"
                    onClick={() => void submitJob("ritual")}
                    disabled={busyAction === "ritual"}
                    style={{
                      fontSize: 11,
                      width: "100%",
                      gridColumn: "1 / -1",
                    }}
                  >
                    Request Sacred Ritual
                  </button>
                </div>

                {/* ─── NICKNAME / SUBSCRIBER ─── */}
                <div style={{ marginTop: 12 }}>
                  <LightRule />
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      fontSize: 10,
                      fontFamily:
                        "var(--font-special-elite, 'Courier New', monospace)",
                      color: INK.inkFaded,
                    }}
                  >
                    <span style={{ whiteSpace: "nowrap" }}>
                      SUBSCRIBER NAME:
                    </span>
                    <input
                      defaultValue={nickname}
                      onBlur={(e) => updateNickname(e.target.value)}
                      style={{
                        background: "transparent",
                        border: "none",
                        borderBottom: `1px solid ${INK.rule}`,
                        color: INK.ink,
                        padding: "2px 4px",
                        fontSize: 11,
                        fontFamily:
                          "var(--font-special-elite, 'Courier New', monospace)",
                        width: 120,
                        fontWeight: 700,
                      }}
                    />
                  </div>
                </div>
              </div>

              {/* Column divider */}
              <div className="ce-col-divider" />

              {/* Right: Society Pages + Obituaries */}
              <div style={{ padding: "0 0 0 14px" }} className="ce-stagger-1">
                <SectionHeader>Society Pages</SectionHeader>
                <div
                  style={{
                    fontFamily:
                      "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                    fontSize: 11,
                    fontStyle: "italic",
                    color: INK.inkFaded,
                    marginBottom: 6,
                  }}
                >
                  Who&apos;s Who in {colony.name} {DIAMOND} {population}{" "}
                  Residents
                </div>

                {aliveCats.length === 0 ? (
                  <div
                    style={{
                      textAlign: "center",
                      fontStyle: "italic",
                      color: INK.inkFaded,
                      padding: "12px 0",
                      fontSize: 12,
                    }}
                  >
                    The colony stands empty. No residents to report.
                  </div>
                ) : (
                  <div>
                    {aliveCats.map((cat: any) => {
                      const isLeader = leader && leader._id === cat._id;
                      const identity = summarizeCatIdentity(
                        cat.spriteParams as Record<string, unknown> | null,
                      );
                      const spec = cat.specialization ?? "unassigned";
                      const isExpanded = expandedCatId === cat._id;

                      return (
                        <div
                          key={cat._id}
                          style={{
                            padding: "5px 0",
                            borderBottom: `1px dotted ${INK.rule}`,
                            cursor: "pointer",
                          }}
                          onClick={() =>
                            setExpandedCatId(isExpanded ? null : cat._id)
                          }
                        >
                          {/* Name + Role */}
                          <div
                            style={{
                              display: "flex",
                              justifyContent: "space-between",
                              alignItems: "baseline",
                            }}
                          >
                            <span
                              style={{
                                fontFamily:
                                  "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                                fontWeight: 700,
                                fontSize: 13,
                                color: isLeader ? INK.red : INK.ink,
                              }}
                            >
                              {cat.name}
                              {isLeader && (
                                <span
                                  style={{
                                    fontSize: 9,
                                    fontWeight: 700,
                                    color: INK.white,
                                    background: INK.red,
                                    padding: "1px 5px",
                                    marginLeft: 6,
                                    letterSpacing: "0.1em",
                                    verticalAlign: "middle",
                                  }}
                                >
                                  LEADER
                                </span>
                              )}
                            </span>
                            <span
                              style={{
                                fontSize: 10,
                                fontFamily:
                                  "var(--font-special-elite, 'Courier New', monospace)",
                                color: INK.inkFaded,
                                textTransform: "capitalize",
                              }}
                            >
                              {spec}
                            </span>
                          </div>

                          {/* Brief bio */}
                          <div
                            style={{
                              fontSize: 11,
                              color: INK.inkLight,
                              lineHeight: 1.4,
                              marginTop: 1,
                            }}
                          >
                            {identity.coat} coat, {identity.eyes.toLowerCase()}{" "}
                            eyes
                            {identity.markings !== "None"
                              ? `, ${identity.markings.toLowerCase()}`
                              : ""}
                            .
                            {cat.currentTask
                              ? ` Currently engaged in ${cat.currentTask.replace(/_/g, " ")}.`
                              : " At leisure."}
                          </div>

                          {/* Expanded details */}
                          {isExpanded && (
                            <div
                              style={{
                                marginTop: 6,
                                padding: "6px 8px",
                                background: `${INK.rule}15`,
                                border: `1px solid ${INK.rule}50`,
                                fontSize: 10,
                                fontFamily:
                                  "var(--font-special-elite, 'Courier New', monospace)",
                                color: INK.inkLight,
                                animation: "ce-fadeIn 200ms ease-out",
                              }}
                            >
                              <div
                                style={{
                                  display: "grid",
                                  gridTemplateColumns: "1fr 1fr",
                                  gap: "2px 12px",
                                }}
                              >
                                <span>Lineage: {identity.lineage}</span>
                                <span>Skin: {identity.skin}</span>
                                <span>Accessories: {identity.accessories}</span>
                                <span>Scars: {identity.scars}</span>
                                <span>Sprite: {identity.sprite}</span>
                                <span>Species: {identity.species}</span>
                              </div>
                              {cat.stats && (
                                <div style={{ marginTop: 4 }}>
                                  ATK:{Math.floor(cat.stats.attack)} DEF:
                                  {Math.floor(cat.stats.defense)} HNT:
                                  {Math.floor(cat.stats.hunting)} MED:
                                  {Math.floor(cat.stats.medicine)} BLD:
                                  {Math.floor(cat.stats.building)} LDR:
                                  {Math.floor(cat.stats.leadership)}
                                </div>
                              )}
                              {cat.roleXp && (
                                <div style={{ marginTop: 2 }}>
                                  XP: Hunter {Math.floor(cat.roleXp.hunter)} /
                                  Architect {Math.floor(cat.roleXp.architect)} /
                                  Ritualist {Math.floor(cat.roleXp.ritualist)}
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* ─── OBITUARIES ─── */}
                {deadCats.length > 0 && (
                  <>
                    <OrnamentalRule />
                    <SectionHeader>Obituaries</SectionHeader>
                    {deadCats.map((cat: any) => {
                      const identity = summarizeCatIdentity(
                        cat.spriteParams as Record<string, unknown> | null,
                      );
                      return (
                        <div
                          key={cat._id}
                          style={{
                            padding: "4px 0",
                            borderBottom: `1px dotted ${INK.rule}`,
                            fontSize: 11,
                            color: INK.inkLight,
                          }}
                        >
                          <span
                            style={{
                              fontFamily:
                                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                              fontWeight: 700,
                            }}
                          >
                            {cat.name}
                          </span>{" "}
                          {DIAMOND}{" "}
                          <span style={{ fontStyle: "italic" }}>
                            Beloved {identity.coat.toLowerCase()} of{" "}
                            {colony.name}. Specialization:{" "}
                            {(cat.specialization ?? "none").replace(/_/g, " ")}.
                            Passed{" "}
                            {cat.deathTime
                              ? formatDate(cat.deathTime)
                              : "recently"}
                            . They will be missed.
                          </span>
                        </div>
                      );
                    })}
                  </>
                )}
              </div>
            </div>
          </section>

          <HeavyRule />

          {/* ═══════════════════════════════════════
           SECTION: NEWS
           ═══════════════════════════════════════ */}
          <section
            id="section-news"
            className={
              revealedSections.has("section-news")
                ? "ce-section-visible"
                : "ce-section-hidden"
            }
            style={{ scrollMarginTop: 48, marginTop: 8 }}
          >
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1px 1fr",
                gap: 0,
              }}
            >
              {/* Left: Weather + Latest Dispatches */}
              <div style={{ padding: "0 16px 0 0" }}>
                <SectionHeader>Colony Weather</SectionHeader>
                <div
                  style={{
                    border: `2px solid ${INK.ink}`,
                    padding: "8px 10px",
                    marginBottom: 10,
                    textAlign: "center",
                  }}
                >
                  <div
                    style={{
                      fontFamily:
                        "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                      fontSize: 22,
                      fontWeight: 900,
                      letterSpacing: "0.05em",
                      textTransform: "uppercase",
                      color:
                        morale > 60
                          ? INK.green
                          : morale > 30
                            ? INK.gold
                            : INK.red,
                      lineHeight: 1.1,
                    }}
                  >
                    {forecast.label}
                  </div>
                  <div
                    style={{
                      fontSize: 11,
                      color: INK.inkFaded,
                      marginTop: 2,
                      fontStyle: "italic",
                    }}
                  >
                    Morale Index: {morale}% {BULLET} Outlook: {forecast.icon}
                  </div>
                  <LightRule />
                  <div
                    style={{
                      fontSize: 11,
                      color: INK.inkLight,
                      lineHeight: 1.4,
                      textAlign: "left",
                    }}
                  >
                    {forecast.desc}
                  </div>
                </div>

                <SectionHeader>Latest Dispatches</SectionHeader>

                {events.length === 0 ? (
                  <div
                    style={{
                      textAlign: "center",
                      fontStyle: "italic",
                      color: INK.inkFaded,
                      padding: "8px 0",
                      fontSize: 11,
                    }}
                  >
                    No dispatches received. The wire is quiet.
                  </div>
                ) : (
                  <div
                    style={{
                      maxHeight: 340,
                      overflowY: "auto",
                    }}
                  >
                    {events.slice(0, 20).map((event: any, i: number) => {
                      const time = formatTime(
                        event.timestamp ?? event._creationTime,
                      );
                      const isCritical = /critical|dead|died|lost/i.test(
                        event.message,
                      );
                      const isPositive =
                        /born|built|complet|thriv|success/i.test(event.message);

                      return (
                        <div
                          key={event._id ?? i}
                          style={{
                            padding: "3px 0",
                            borderBottom:
                              i < 19 ? `1px dotted ${INK.rule}60` : "none",
                            fontSize: 10,
                            lineHeight: 1.4,
                          }}
                        >
                          <span
                            style={{
                              fontFamily:
                                "var(--font-special-elite, 'Courier New', monospace)",
                              color: INK.inkFaded,
                              fontSize: 9,
                            }}
                          >
                            {time}
                          </span>{" "}
                          <span
                            style={{
                              color: isCritical
                                ? INK.red
                                : isPositive
                                  ? INK.green
                                  : INK.inkLight,
                            }}
                          >
                            {event.message}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Column divider */}
              <div className="ce-col-divider" />

              {/* Right: Advertisements (Upgrades) */}
              <div style={{ padding: "0 0 0 14px" }} className="ce-stagger-1">
                <SectionHeader>Advertisements</SectionHeader>

                {upgrades.length === 0 ? (
                  <div
                    style={{
                      textAlign: "center",
                      fontStyle: "italic",
                      color: INK.inkFaded,
                      padding: "8px 0",
                      fontSize: 11,
                    }}
                  >
                    No advertisements placed this edition.
                  </div>
                ) : (
                  <div>
                    {upgrades.map((upgrade: any) => {
                      const cost = upgrade.baseCost * (upgrade.level + 1);
                      const maxed = upgrade.level >= upgrade.maxLevel;
                      const canAfford = ritualPoints >= cost;
                      const label = upgrade.key.replace(/_/g, " ");

                      return (
                        <div
                          key={upgrade._id ?? upgrade.key}
                          style={{
                            border: `1.5px solid ${INK.ink}`,
                            padding: "6px 8px",
                            marginBottom: 6,
                            position: "relative",
                            background: maxed ? `${INK.rule}15` : INK.paper,
                          }}
                        >
                          {/* Ad header */}
                          <div
                            style={{
                              fontFamily:
                                "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                              fontWeight: 900,
                              fontSize: 14,
                              textTransform: "uppercase",
                              textAlign: "center",
                              letterSpacing: "0.08em",
                              lineHeight: 1.1,
                              color: maxed ? INK.inkFaded : INK.ink,
                            }}
                          >
                            {DIAMOND} {label} {DIAMOND}
                          </div>
                          {/* Description */}
                          <div
                            style={{
                              fontSize: 10,
                              textAlign: "center",
                              color: INK.inkLight,
                              fontStyle: "italic",
                              margin: "3px 0",
                              lineHeight: 1.3,
                            }}
                          >
                            {upgrade.description ??
                              `Improve your colony's ${label} capabilities`}
                          </div>
                          {/* Level + Cost */}
                          <div
                            style={{
                              display: "flex",
                              justifyContent: "space-between",
                              alignItems: "center",
                              fontSize: 10,
                              fontFamily:
                                "var(--font-special-elite, 'Courier New', monospace)",
                              marginTop: 4,
                            }}
                          >
                            <span style={{ color: INK.inkFaded }}>
                              Lv.{upgrade.level}/{upgrade.maxLevel}
                            </span>
                            {maxed ? (
                              <span
                                style={{
                                  fontWeight: 700,
                                  color: INK.inkFaded,
                                  fontStyle: "italic",
                                }}
                              >
                                SOLD OUT
                              </span>
                            ) : (
                              <button
                                className={`ce-btn ${canAfford ? "ce-btn-green" : "ce-btn-red"}`}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  void onBuyUpgrade(upgrade.key);
                                }}
                                disabled={
                                  maxed ||
                                  !canAfford ||
                                  busyAction === `upgrade:${upgrade.key}`
                                }
                                style={{
                                  fontSize: 9,
                                  padding: "2px 8px",
                                  borderWidth: 1,
                                }}
                              >
                                Buy ({cost} RP)
                              </button>
                            )}
                          </div>
                          {/* Level progress dots */}
                          <div
                            style={{
                              display: "flex",
                              justifyContent: "center",
                              gap: 3,
                              marginTop: 4,
                            }}
                          >
                            {Array.from({ length: upgrade.maxLevel }).map(
                              (_, i) => (
                                <div
                                  key={i}
                                  style={{
                                    width: 8,
                                    height: 8,
                                    borderRadius: "50%",
                                    border: `1px solid ${INK.ink}`,
                                    background:
                                      i < upgrade.level
                                        ? INK.ink
                                        : "transparent",
                                  }}
                                />
                              ),
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          </section>

          <HeavyRule />

          {/* ═══════════════════════════════════════
           SECTION: EXTRAS
           ═══════════════════════════════════════ */}
          <section
            id="section-extras"
            className={
              revealedSections.has("section-extras")
                ? "ce-section-visible"
                : "ce-section-hidden"
            }
            style={{ scrollMarginTop: 48, marginTop: 8 }}
          >
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1fr 1fr",
                gap: 16,
              }}
            >
              {/* Amusements */}
              <div
                style={{
                  border: `2px solid ${INK.ink}`,
                  padding: "8px 10px",
                }}
              >
                <div
                  style={{
                    fontFamily:
                      "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                    fontWeight: 900,
                    fontSize: 13,
                    textTransform: "uppercase",
                    textAlign: "center",
                    letterSpacing: "0.15em",
                    marginBottom: 4,
                  }}
                >
                  Amusements
                </div>
                <LightRule />
                {/* Mini crossword grid - decorative */}
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(5, 1fr)",
                    gap: 1,
                    maxWidth: 100,
                    margin: "6px auto",
                  }}
                >
                  {[
                    "C",
                    "A",
                    "T",
                    "S",
                    " ",
                    " ",
                    "I",
                    "D",
                    "L",
                    "E",
                    "M",
                    "E",
                    "O",
                    "W",
                    " ",
                    " ",
                    "P",
                    "U",
                    "R",
                    "R",
                    "N",
                    "A",
                    "P",
                    " ",
                    " ",
                  ].map((ch, i) => (
                    <div
                      key={i}
                      style={{
                        width: 18,
                        height: 18,
                        border: ch === " " ? "none" : `1px solid ${INK.ink}`,
                        background: ch === " " ? INK.ink : INK.paper,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        fontFamily:
                          "var(--font-special-elite, 'Courier New', monospace)",
                        fontSize: 10,
                        fontWeight: 700,
                      }}
                    >
                      {ch !== " " ? ch : ""}
                    </div>
                  ))}
                </div>
                <div
                  style={{
                    textAlign: "center",
                    fontSize: 9,
                    color: INK.inkFaded,
                    fontStyle: "italic",
                    marginTop: 4,
                  }}
                >
                  Today&apos;s Puzzle: 5 letters, &ldquo;Feline pastime&rdquo;
                </div>
              </div>

              {/* Editorial Cartoon */}
              <div
                style={{
                  padding: "6px 8px",
                  border: `1px solid ${INK.rule}`,
                  textAlign: "center",
                }}
                className="ce-stagger-1"
              >
                <div
                  style={{
                    fontFamily:
                      "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                    fontSize: 10,
                    letterSpacing: "0.15em",
                    textTransform: "uppercase",
                    color: INK.inkFaded,
                    fontWeight: 700,
                    marginBottom: 4,
                  }}
                >
                  Editorial Cartoon
                </div>
                <div
                  style={{
                    fontFamily:
                      "var(--font-special-elite, 'Courier New', monospace)",
                    fontSize: 20,
                    lineHeight: 1.2,
                    letterSpacing: "0.1em",
                    whiteSpace: "pre",
                    color: INK.inkLight,
                  }}
                >
                  {(
                    {
                      thriving: `  /\\_/\\  \n ( o.o ) \n  > ^ <  \n "Purrfect!"`,
                      struggling: `  /\\_/\\  \n ( T_T ) \n  > ~ <  \n  "Mrow..."`,
                      dead: `  /\\_/\\  \n ( x_x ) \n  > - <  \n  "...zzz"`,
                    } as Record<string, string>
                  )[colony.status] ??
                    `  /\\_/\\  \n ( ?.? ) \n  > ^ <  \n  "Hmm?"`}
                </div>
              </div>

              {/* Pull Quote */}
              <div
                style={{
                  padding: "8px 12px",
                  borderLeft: `3px solid ${INK.ink}`,
                  borderRight: `3px solid ${INK.ink}`,
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "center",
                }}
              >
                <div
                  style={{
                    fontFamily:
                      "var(--font-playfair, Georgia, 'Times New Roman', serif)",
                    fontWeight: 700,
                    fontSize: 15,
                    fontStyle: "italic",
                    lineHeight: 1.3,
                    textAlign: "center",
                    color: INK.inkLight,
                  }}
                >
                  &ldquo;A colony is only as strong as the kibble in its stores
                  and the purrs in its hearts.&rdquo;
                </div>
                <div
                  style={{
                    textAlign: "right",
                    fontSize: 10,
                    color: INK.inkFaded,
                    marginTop: 4,
                    fontFamily:
                      "var(--font-special-elite, 'Courier New', monospace)",
                  }}
                >
                  {ORNAMENT} The Editor
                </div>
              </div>
            </div>
          </section>
        </div>
        {/* end ce-content-wrap */}

        {/* ═══════════════════════════════════════
           FOOTER / COLOPHON
           ═══════════════════════════════════════ */}
        <div style={{ marginTop: 16 }}>
          <div
            style={{
              borderTop: `2px solid ${INK.ink}`,
              borderBottom: `4px double ${INK.ink}`,
              padding: "6px 0",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              fontSize: 9,
              fontFamily: "var(--font-special-elite, 'Courier New', monospace)",
              color: INK.inkFaded,
              letterSpacing: "0.08em",
              textTransform: "uppercase",
            }}
          >
            <span>
              Published continuously since the founding of {colony.name}
            </span>
            <span>
              {ORNAMENT} The Catford Examiner {ORNAMENT}
            </span>
            <span>
              {onlineCount} reader{onlineCount !== 1 ? "s" : ""} {DIAMOND}{" "}
              {formatTime(now)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
