// Original backdrop art (no game assets): a dusk sky over rolling hills with a glowing orb split light and dark.

export function Backdrop({ className }: { className?: string }) {
  return (
    <div aria-hidden className={`pointer-events-none absolute inset-0 overflow-hidden ${className ?? ""}`}>
      <svg className="absolute inset-0 size-full" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMid slice">
        <defs>
          <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#0d0b09" />
            <stop offset="0.55" stopColor="#2a1f14" />
            <stop offset="1" stopColor="#4a3220" />
          </linearGradient>
          <radialGradient id="sun" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="#f0d48a" stopOpacity="0.55" />
            <stop offset="1" stopColor="#f0d48a" stopOpacity="0" />
          </radialGradient>
          <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0.35" stopColor="#0d0b09" stopOpacity="0" />
            <stop offset="1" stopColor="#0d0b09" stopOpacity="1" />
          </linearGradient>
        </defs>
        <rect width="1600" height="900" fill="url(#sky)" />
        <circle
          className="animate-glow"
          style={{ transformOrigin: "1250px 300px" }}
          cx="1250"
          cy="300"
          r="300"
          fill="url(#sun)"
        />
        <g transform="translate(1250 300) scale(0.8)">
          <circle r="86" fill="#f4efe4" opacity="0.92" />
          <path d="M0 -86a86 86 0 0 0 0 172z" fill="#16130f" opacity="0.92" />
          <path d="M0 -58c-22 22 -22 94 0 116c22 -22 22 -94 0 -116z" fill="#16130f" opacity="0.92" />
          <path d="M0 -58c-22 22 -22 94 0 116" fill="#f4efe4" opacity="0.92" />
        </g>
        <path d="M0 640 C 220 560 380 600 560 560 S 900 470 1120 540 S 1450 600 1600 560 V900 H0Z" fill="#1d1610" />
        <path d="M0 720 C 260 650 470 700 700 660 S 1100 620 1300 680 S 1520 720 1600 700 V900 H0Z" fill="#15110c" />
        <path d="M0 800 C 300 760 600 790 900 770 S 1400 760 1600 790 V900 H0Z" fill="#0f0c09" />
        <rect width="1600" height="900" fill="url(#fade)" />
      </svg>
    </div>
  );
}
