-- Drop NULLS NOT DISTINCT constraint that prevents multiple anonymous submissions
-- (anonymous surveys use respondent_id = NULL for all rows, so NULLS NOT DISTINCT
--  blocks every submission after the first)
ALTER TABLE survey_responses
    DROP CONSTRAINT survey_responses_survey_id_respondent_id_key;

-- Partial unique index: one response per authenticated user per survey only
CREATE UNIQUE INDEX survey_responses_unique_auth_respondent
    ON survey_responses (survey_id, respondent_id)
    WHERE respondent_id IS NOT NULL;
