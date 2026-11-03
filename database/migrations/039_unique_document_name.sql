ALTER TABLE document_requests
  ADD CONSTRAINT unique_document_name_per_school_audience
  UNIQUE (school_id, audience, document_name);
