import { redirect } from "next/navigation";

// The Catford Examiner is the production UI; the experimental dashboard
// this page used to host was retired with the Convex → SQLite migration.
export default function GamePage() {
	redirect("/game/newspaper");
}
