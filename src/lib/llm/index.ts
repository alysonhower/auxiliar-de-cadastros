import { Anthropic } from "@anthropic-ai/sdk";
import { z } from "zod";

const client = new Anthropic();

const Config = {
    TEMPERATURE: 0.0,
    TOP_P: 0.0,
    TOP_K: 0,
    MAX_TOKENS: 4096,
} as const;

enum Model {
    Haiku = "claude-3-haiku-20240307",
    Sonnet = "claude-3-5-sonnet-20240620",
}