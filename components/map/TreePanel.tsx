"use client";

import {
	prerequisitesMet,
	UPGRADE_NODES,
	type UpgradeEra,
	type UpgradeNode,
} from "@/lib/game/upgradeTree";

const ERAS: { era: UpgradeEra; label: string }[] = [
	{ era: 1, label: "Era I · Foundations" },
	{ era: 2, label: "Era II · Craft & Growth" },
	{ era: 3, label: "Era III · Advanced" },
];

type NodeState = "owned" | "unlockable" | "locked";

function nodeState(node: UpgradeNode, owned: Set<string>): NodeState {
	if (owned.has(node.id)) {
		return "owned";
	}
	if (
		prerequisitesMet({ ownedNodeIds: [...owned], researchPoints: 0 }, node.id)
	) {
		return "unlockable";
	}
	return "locked";
}

export interface ResearchSummary {
	ownedNodeIds: string[];
	researchPoints: number;
	researcherCount: number;
	blessings: number;
	nextTarget: { id: string; name: string; cost: number } | null;
}

export function TreePanel({
	research,
	busyAction,
	onUnlockNode,
	onClose,
}: {
	research: ResearchSummary;
	busyAction: string | null;
	onUnlockNode: (nodeId: string) => void;
	onClose: () => void;
}) {
	const owned = new Set(research.ownedNodeIds);
	const blessings = research.blessings;
	const target = research.nextTarget;
	const progress = target
		? Math.min(100, (research.researchPoints / target.cost) * 100)
		: 0;

	return (
		<div className="absolute inset-0 z-40 flex items-center justify-center bg-black/50 p-4">
			<div className="flex max-h-full w-full max-w-4xl flex-col overflow-hidden rounded-lg border-2 border-[#5d4024] bg-[#f3e6c8] shadow-[0_4px_24px_rgba(0,0,0,0.6)]">
				{/* Header */}
				<div className="flex items-center justify-between border-b-2 border-[#5d4024] bg-gradient-to-b from-[#4a3319] to-[#3a2712] px-4 py-2">
					<h2 className="font-serif text-lg font-black text-amber-100">
						🌳 Upgrade Tree
					</h2>
					<div className="flex items-center gap-3 text-xs font-bold text-amber-100">
						<span title="Blessings — spent on god purchases">
							✨ {Math.floor(blessings)} blessings
						</span>
						<span title="Research points accrued by the cats">
							🔬 {research.researchPoints.toFixed(1)} pts ·{" "}
							{research.researcherCount} researcher
							{research.researcherCount === 1 ? "" : "s"}
						</span>
						<button
							type="button"
							onClick={onClose}
							className="rounded px-2 text-base font-bold text-amber-100 hover:bg-white/10"
							aria-label="Close upgrade tree"
						>
							✕
						</button>
					</div>
				</div>

				{/* Research progress toward the cats' next auto-unlock */}
				<div className="border-b border-[#5d4024]/40 px-4 py-2">
					{target ? (
						<div className="text-xs font-semibold text-[#4a3319]">
							<div className="mb-1 flex justify-between">
								<span>
									Cats researching:{" "}
									<span className="font-bold">{target.name}</span>
								</span>
								<span className="text-[#6b4a2a]">
									{research.researchPoints.toFixed(1)} / {target.cost}
								</span>
							</div>
							<div className="h-2 overflow-hidden rounded-full border border-[#5d4024]/50 bg-black/10">
								<div
									className="h-full bg-gradient-to-r from-emerald-500 to-emerald-600"
									style={{ width: `${progress}%` }}
								/>
							</div>
						</div>
					) : (
						<p className="text-xs font-semibold text-[#6b4a2a]">
							Everything researchable is unlocked — the gods must open the next
							path.
						</p>
					)}
				</div>

				{/* Eras as columns */}
				<div className="grid grid-cols-1 gap-3 overflow-y-auto p-4 sm:grid-cols-3">
					{ERAS.map(({ era, label }) => (
						<div key={era} className="flex flex-col gap-2">
							<h3 className="font-serif text-xs font-bold uppercase tracking-wider text-[#6b4a2a]">
								{label}
							</h3>
							{UPGRADE_NODES.filter((node) => node.era === era).map((node) => {
								const state = nodeState(node, owned);
								const affordable = blessings >= node.cost;
								const buyable = state === "unlockable" && affordable;
								return (
									<div
										key={node.id}
										className={`rounded border p-2 text-xs ${
											state === "owned"
												? "border-emerald-700 bg-emerald-100/60"
												: state === "unlockable"
													? "border-amber-700 bg-amber-50"
													: "border-[#5d4024]/40 bg-[#e9d9b4]/50 opacity-70"
										}`}
									>
										<div className="flex items-center justify-between gap-2">
											<span className="font-bold text-[#3a2712]">
												{state === "owned" && "✅ "}
												{node.name}
											</span>
											{state === "owned" ? (
												<span className="rounded bg-emerald-700 px-1.5 py-0.5 text-[10px] font-bold text-white">
													OWNED
												</span>
											) : (
												<button
													type="button"
													disabled={
														!buyable || busyAction === `unlock:${node.id}`
													}
													onClick={() => onUnlockNode(node.id)}
													title={
														state === "locked"
															? "Prerequisites not met"
															: affordable
																? "Spend blessings to unlock now"
																: "Not enough blessings"
													}
													className="rounded border border-amber-700 bg-gradient-to-b from-amber-400 to-amber-500 px-2 py-0.5 font-bold text-[#3a2712] shadow hover:from-amber-300 disabled:opacity-40"
												>
													{node.cost} ✨
												</button>
											)}
										</div>
										<p className="mt-1 text-[11px] leading-tight text-[#4a3319]">
											{node.description}
										</p>
									</div>
								);
							})}
						</div>
					))}
				</div>
			</div>
		</div>
	);
}
