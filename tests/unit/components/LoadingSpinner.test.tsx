/**
 * Tests for LoadingSpinner Component
 */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LoadingSpinner } from "@/components/ui/LoadingSpinner";

describe("LoadingSpinner", () => {
	it("should render spinner", () => {
		const { container } = render(<LoadingSpinner />);
		const spinner = container.querySelector('[role="status"]');
		expect(spinner).toBeInTheDocument();
	});

	it("should display text when provided", () => {
		render(<LoadingSpinner text="Loading..." />);
		expect(screen.getByText("Loading...")).toBeInTheDocument();
	});

	it("should apply size classes", () => {
		const { container } = render(<LoadingSpinner size="large" />);
		const spinner = container.querySelector('[role="status"]');
		expect(spinner).toHaveClass("w-12", "h-12");
	});

	it("should have correct aria-label", () => {
		const { container } = render(<LoadingSpinner />);
		const spinner = container.querySelector('[role="status"]');
		expect(spinner).toHaveAttribute("aria-label", "Loading");
	});
});
