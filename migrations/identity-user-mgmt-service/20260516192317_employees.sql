-- Migration: employees
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.employees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    employee_id VARCHAR(64) NOT NULL,
    department VARCHAR(255),
    title VARCHAR(255),
    manager_id UUID REFERENCES sesame_idam.users(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
