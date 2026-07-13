-- Migration: organizations
-- Generated: 20260713065042

ALTER TABLE organizations ADD COLUMN IF NOT EXISTS metadata JSONB;
