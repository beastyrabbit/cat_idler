"use client";

import { useState } from "react";
import { useQuery, useMutation } from "convex/react";
import { api } from "@/convex/_generated/api";
import { useRouter } from "next/navigation";
import Link from "next/link";

export default function Home() {
  const router = useRouter();
  const colonies = useQuery(api.colonies.getAllColonies);
  const createColony = useMutation(api.colonies.createColony);
  const [newColonyName, setNewColonyName] = useState("");
  const [isCreating, setIsCreating] = useState(false);

  const handleCreateColony = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newColonyName.trim() || isCreating) return;

    setIsCreating(true);
    try {
      const colonyId = await createColony({
        name: newColonyName.trim(),
        leaderId: null,
      });
      router.push(`/colony/${colonyId}`);
    } catch (error) {
      console.error("Failed to create colony:", error);
      setIsCreating(false);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "thriving":
        return "bg-green-500";
      case "starting":
        return "bg-blue-500";
      case "struggling":
        return "bg-yellow-500";
      case "dead":
        return "bg-gray-500";
      default:
        return "bg-gray-400";
    }
  };

  const getStatusText = (status: string) => {
    switch (status) {
      case "thriving":
        return "Thriving";
      case "starting":
        return "Starting";
      case "struggling":
        return "Struggling";
      case "dead":
        return "Dead";
      default:
        return status;
    }
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-amber-50 via-orange-50 to-yellow-100">
      {/* Background pattern */}
      <div className="fixed inset-0 pointer-events-none opacity-5 bg-[url('/images/backgrounds/patterns/paw-print.png')] bg-repeat" />

      <div className="relative max-w-4xl mx-auto px-4 py-12">
        {/* Header */}
        <div className="text-center mb-12">
          <h1 className="text-5xl font-bold text-amber-900 mb-4 tracking-tight">
            🐱 Cat Colony Idle Game
          </h1>
          <p className="text-lg text-amber-700 max-w-2xl mx-auto">
            Build and manage your own cat colony! Watch your cats hunt, gather,
            build, and thrive. Help them survive encounters and grow your colony
            from a small den to a thriving community.
          </p>
        </div>

        {/* Create New Colony */}
        <div className="bg-white/80 backdrop-blur-sm rounded-2xl shadow-xl p-8 mb-8 border border-amber-200">
          <h2 className="text-2xl font-semibold text-amber-900 mb-4">
            Create New Colony
          </h2>
          <form onSubmit={handleCreateColony} className="flex gap-4">
            <input
              type="text"
              value={newColonyName}
              onChange={(e) => setNewColonyName(e.target.value)}
              placeholder="Colony name"
              className="flex-1 px-4 py-3 rounded-xl border-2 border-amber-200 focus:border-amber-400 focus:outline-none bg-white text-amber-900 placeholder-amber-400 transition-colors"
              disabled={isCreating}
            />
            <button
              type="submit"
              disabled={!newColonyName.trim() || isCreating}
              className="px-8 py-3 bg-gradient-to-r from-amber-500 to-orange-500 text-white font-semibold rounded-xl shadow-lg hover:from-amber-600 hover:to-orange-600 disabled:opacity-50 disabled:cursor-not-allowed transition-all hover:shadow-xl active:scale-95"
            >
              {isCreating ? "Creating..." : "Create Colony"}
            </button>
          </form>
        </div>

        {/* Existing Colonies */}
        <div className="bg-white/80 backdrop-blur-sm rounded-2xl shadow-xl p-8 border border-amber-200">
          <h2 className="text-2xl font-semibold text-amber-900 mb-6">
            Your Colonies
          </h2>

          {colonies === undefined ? (
            <div className="text-center py-12">
              <div className="animate-spin w-8 h-8 border-4 border-amber-300 border-t-amber-600 rounded-full mx-auto mb-4" />
              <p className="text-amber-600">Loading colonies...</p>
            </div>
          ) : colonies.length === 0 ? (
            <div className="text-center py-12">
              <div className="text-6xl mb-4">🏕️</div>
              <p className="text-amber-700 text-lg">
                No colonies yet. Create your first colony above!
              </p>
            </div>
          ) : (
            <div className="grid gap-4">
              {colonies.map((colony) => (
                <Link
                  key={colony._id}
                  href={`/colony/${colony._id}`}
                  className="block group"
                >
                  <div className="flex items-center justify-between p-5 bg-gradient-to-r from-amber-50 to-orange-50 rounded-xl border-2 border-amber-200 hover:border-amber-400 hover:shadow-lg transition-all group-hover:scale-[1.01]">
                    <div className="flex items-center gap-4">
                      <div className="text-3xl">
                        {colony.status === "dead" ? "💀" : "🏠"}
                      </div>
                      <div>
                        <h3 className="text-xl font-semibold text-amber-900 group-hover:text-amber-700">
                          {colony.name}
                        </h3>
                        <p className="text-sm text-amber-600">
                          Created{" "}
                          {new Date(colony.createdAt).toLocaleDateString()}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-4">
                      {/* Resources preview */}
                      <div className="hidden sm:flex items-center gap-3 text-sm text-amber-700">
                        <span title="Food">🍖 {colony.resources.food}</span>
                        <span title="Water">💧 {colony.resources.water}</span>
                        <span title="Herbs">🌿 {colony.resources.herbs}</span>
                      </div>
                      {/* Status badge */}
                      <div
                        className={`px-3 py-1 rounded-full text-white text-sm font-medium ${getStatusColor(colony.status)}`}
                      >
                        {getStatusText(colony.status)}
                      </div>
                      {/* Arrow */}
                      <div className="text-amber-400 group-hover:text-amber-600 group-hover:translate-x-1 transition-all">
                        →
                      </div>
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="text-center mt-12 text-amber-600 text-sm">
          <p>
            The colony runs autonomously every 10 seconds. Help your cats
            thrive!
          </p>
        </div>
      </div>
    </main>
  );
}
