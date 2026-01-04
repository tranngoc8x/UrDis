import { genkit, z } from "genkit";
import { googleAI } from "@genkit-ai/google-genai";

// Initialize the Genkit SDK
export const ai = genkit({
  plugins: [googleAI()],
  model: "googleai/gemini-1.5-flash", // Set default model
});

// Define a simple flow
export const menuSuggestionFlow = ai.defineFlow(
  {
    name: "menuSuggestionFlow",
    inputSchema: z.string().describe("The theme of the restaurant"),
    outputSchema: z.string().describe("A suggested menu item"),
  },
  async (theme) => {
    const response = await ai.generate(
      `Suggest a menu item for a ${theme} themed restaurant.`
    );
    return response.text;
  }
);
