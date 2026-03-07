import type { Metadata } from "next";
import { redirect } from "next/navigation";

export const metadata: Metadata = {
	title: "Cat Colony Idle",
	description:
		"Cat Colony Idle is a cozy, persistent cat simulation with colony management.",
};

export default function HomePage() {
	redirect("/game");
}
