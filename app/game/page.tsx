import { MapScreen } from "@/components/map/MapScreen";

// Map-first game screen; The Catford Examiner remains available at
// /game/newspaper and via the in-map News drawer.
export default function GamePage() {
	return <MapScreen renderer="pixi" />;
}
