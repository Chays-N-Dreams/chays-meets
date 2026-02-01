-- Global schema: Application-wide settings, API keys, licensing, and custom OpenAI config.
-- This is a consolidated schema combining all incremental migrations into a single CREATE set.

-- Create settings table
CREATE TABLE IF NOT EXISTS settings (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    whisperModel TEXT NOT NULL,
    groqApiKey TEXT,
    openaiApiKey TEXT,
    anthropicApiKey TEXT,
    ollamaApiKey TEXT,
    openRouterApiKey TEXT,
    ollamaEndpoint TEXT,
    customOpenAIConfig TEXT,
    geminiApiKey TEXT
);

-- Create transcript_settings table
CREATE TABLE IF NOT EXISTS transcript_settings (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    whisperApiKey TEXT,
    deepgramApiKey TEXT,
    elevenLabsApiKey TEXT,
    groqApiKey TEXT,
    openaiApiKey TEXT
);

-- Create licensing table
CREATE TABLE IF NOT EXISTS licensing (
    license_key TEXT PRIMARY KEY,
    encrypted_key TEXT NOT NULL,
    signature_hash TEXT NOT NULL,
    activation_date TEXT NOT NULL,
    expiry_date TEXT NOT NULL,
    soft_expiry_date TEXT NOT NULL,
    max_activation_time TEXT NOT NULL,
    duration INTEGER NOT NULL,
    generated_on TEXT NOT NULL,
    is_soft_expired INTEGER DEFAULT 0,
    grace_period INTEGER NOT NULL DEFAULT 604800
);

