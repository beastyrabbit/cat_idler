import { describe, expect, it } from "vitest";

import {
  getWeather,
  getWeatherModifiers,
  getWeatherForecast,
  WEATHER_TYPES,
  type WeatherType,
  type WeatherModifiers,
} from "@/lib/game/weather";

describe("getWeather", () => {
  it("returns a valid WeatherType", () => {
    const weather = getWeather(42, 1);
    expect(WEATHER_TYPES).toContain(weather);
  });

  it("is deterministic — same seed + day always produces same result", () => {
    const a = getWeather(12345, 10);
    const b = getWeather(12345, 10);
    expect(a).toBe(b);
  });

  it("produces different weather for different days", () => {
    const results = new Set<WeatherType>();
    for (let day = 0; day < 100; day++) {
      results.add(getWeather(42, day));
    }
    // Over 100 days, should see more than one weather type
    expect(results.size).toBeGreaterThan(1);
  });

  it("produces different weather for different seeds", () => {
    const results = new Set<WeatherType>();
    for (let seed = 1; seed <= 100; seed++) {
      results.add(getWeather(seed, 5));
    }
    expect(results.size).toBeGreaterThan(1);
  });

  it("handles seed 0 gracefully (normalized to 1)", () => {
    const weather = getWeather(0, 1);
    expect(WEATHER_TYPES).toContain(weather);
  });

  it("handles negative day numbers gracefully", () => {
    const weather = getWeather(42, -5);
    expect(WEATHER_TYPES).toContain(weather);
  });
});

describe("getWeatherModifiers", () => {
  it("returns neutral modifiers for sunny weather", () => {
    const mods = getWeatherModifiers("sunny");
    expect(mods.food).toBe(1.0);
    expect(mods.water).toBe(1.0);
    expect(mods.herbs).toBe(1.0);
  });

  it("returns rainy modifiers — water boost, food penalty", () => {
    const mods = getWeatherModifiers("rainy");
    expect(mods.water).toBeCloseTo(1.3);
    expect(mods.food).toBeCloseTo(0.9);
  });

  it("returns stormy modifiers — big water boost, food and herb penalty", () => {
    const mods = getWeatherModifiers("stormy");
    expect(mods.food).toBeCloseTo(0.7);
    expect(mods.herbs).toBeCloseTo(0.8);
    expect(mods.water).toBeCloseTo(1.5);
  });

  it("returns foggy modifiers — hunting penalty, herb boost", () => {
    const mods = getWeatherModifiers("foggy");
    expect(mods.food).toBeCloseTo(0.8);
    expect(mods.herbs).toBeCloseTo(1.1);
  });

  it("returns windy modifiers — all gathering reduced", () => {
    const mods = getWeatherModifiers("windy");
    expect(mods.food).toBeCloseTo(0.9);
    expect(mods.water).toBeCloseTo(0.9);
    expect(mods.herbs).toBeCloseTo(0.9);
  });

  it("returns drought modifiers — water penalty, food boost", () => {
    const mods = getWeatherModifiers("drought");
    expect(mods.water).toBeCloseTo(0.6);
    expect(mods.food).toBeCloseTo(1.1);
  });

  it("all weather types have food, water, and herbs keys", () => {
    for (const weather of WEATHER_TYPES) {
      const mods = getWeatherModifiers(weather);
      expect(mods).toHaveProperty("food");
      expect(mods).toHaveProperty("water");
      expect(mods).toHaveProperty("herbs");
      expect(typeof mods.food).toBe("number");
      expect(typeof mods.water).toBe("number");
      expect(typeof mods.herbs).toBe("number");
    }
  });

  it("all modifiers are positive numbers", () => {
    for (const weather of WEATHER_TYPES) {
      const mods = getWeatherModifiers(weather);
      expect(mods.food).toBeGreaterThan(0);
      expect(mods.water).toBeGreaterThan(0);
      expect(mods.herbs).toBeGreaterThan(0);
    }
  });
});

describe("getWeatherForecast", () => {
  it("returns the requested number of days", () => {
    const forecast = getWeatherForecast(42, 1, 5);
    expect(forecast).toHaveLength(5);
  });

  it("each entry has day and weather fields", () => {
    const forecast = getWeatherForecast(42, 1, 3);
    for (const entry of forecast) {
      expect(entry).toHaveProperty("day");
      expect(entry).toHaveProperty("weather");
      expect(typeof entry.day).toBe("number");
      expect(WEATHER_TYPES).toContain(entry.weather);
    }
  });

  it("starts from the given day number", () => {
    const forecast = getWeatherForecast(42, 10, 3);
    expect(forecast[0].day).toBe(10);
    expect(forecast[1].day).toBe(11);
    expect(forecast[2].day).toBe(12);
  });

  it("is deterministic — same inputs produce same forecast", () => {
    const a = getWeatherForecast(99, 5, 7);
    const b = getWeatherForecast(99, 5, 7);
    expect(a).toEqual(b);
  });

  it("each day matches individual getWeather calls", () => {
    const seed = 42;
    const startDay = 0;
    const forecast = getWeatherForecast(seed, startDay, 10);
    for (const entry of forecast) {
      expect(entry.weather).toBe(getWeather(seed, entry.day));
    }
  });

  it("returns empty array for 0 days", () => {
    const forecast = getWeatherForecast(42, 1, 0);
    expect(forecast).toEqual([]);
  });
});
