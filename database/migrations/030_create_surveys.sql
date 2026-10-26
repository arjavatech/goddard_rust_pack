-- Surveys feature: surveys, questions, responses, and answers

CREATE TABLE surveys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    audience VARCHAR(16) NOT NULL CHECK (audience IN ('parents','employees','all_staff','classroom','everyone')),
    classroom_id UUID REFERENCES classrooms(id) ON DELETE SET NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published','closed','expired')),
    is_anonymous BOOLEAN NOT NULL DEFAULT FALSE,
    is_template BOOLEAN NOT NULL DEFAULT FALSE,
    total_targeted INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE TABLE survey_questions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    survey_id UUID NOT NULL REFERENCES surveys(id) ON DELETE CASCADE,
    question_type VARCHAR(16) NOT NULL CHECK (question_type IN ('single_choice','multiple_choice','text','rating')),
    question_text TEXT NOT NULL,
    is_required BOOLEAN NOT NULL DEFAULT FALSE,
    options JSONB,
    rating_max INTEGER,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- respondent_id is nullable to support anonymous surveys
CREATE TABLE survey_responses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    survey_id UUID NOT NULL REFERENCES surveys(id) ON DELETE CASCADE,
    respondent_id UUID REFERENCES users(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE NULLS NOT DISTINCT (survey_id, respondent_id)
);

CREATE TABLE survey_response_answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    response_id UUID NOT NULL REFERENCES survey_responses(id) ON DELETE CASCADE,
    question_id UUID NOT NULL REFERENCES survey_questions(id) ON DELETE CASCADE,
    answer_value TEXT,
    answer_values JSONB
);

CREATE INDEX idx_surveys_school_status   ON surveys(school_id, status);
CREATE INDEX idx_surveys_school_audience ON surveys(school_id, audience);
CREATE INDEX idx_survey_questions_survey ON survey_questions(survey_id, sort_order);
CREATE INDEX idx_survey_responses_survey ON survey_responses(survey_id);
CREATE INDEX idx_survey_responses_resp   ON survey_responses(respondent_id, survey_id);
CREATE INDEX idx_survey_answers_response ON survey_response_answers(response_id);
CREATE INDEX idx_survey_answers_question ON survey_response_answers(question_id);
