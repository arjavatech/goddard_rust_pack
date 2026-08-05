-- =============================================
-- Goddard School Enrollment Management System
-- Complete Database Schema
-- Exported from Supabase PostgreSQL
-- Date: 2026-02-10
-- =============================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";


-- =============================================
-- TABLES
-- =============================================

-- Table: schools
 CREATE TABLE schools (                          
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     name VARCHAR(255) NOT NULL,                 
     subdomain VARCHAR(100) NOT NULL,            
     settings JSONB,                             
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: users
 CREATE TABLE users (                            
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     first_name VARCHAR(100) NOT NULL,           
     last_name VARCHAR(100) NOT NULL,            
     email VARCHAR(255) NOT NULL,                
     role VARCHAR(50) NOT NULL,                  
     is_verified BOOLEAN,                        
     created_by UUID,                            
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP,                       
     metadata JSONB,                             
     is_active BOOLEAN DEFAULT true              
 );


-- Table: children
 CREATE TABLE children (                                    
     id UUID DEFAULT uuid_generate_v4() NOT NULL,           
     parent_id UUID NOT NULL,                               
     secondary_parent_id UUID,                              
     school_id UUID NOT NULL,                               
     first_name VARCHAR(100) NOT NULL,                      
     last_name VARCHAR(100) NOT NULL,                       
     birth_date DATE,                                       
     gender VARCHAR(20),                                    
     status VARCHAR(50) DEFAULT 'active'::character varying,
     is_active BOOLEAN DEFAULT true,                        
     created_at TIMESTAMP DEFAULT now(),                    
     updated_at TIMESTAMP                                   
 );


-- Table: classrooms
 CREATE TABLE classrooms (                       
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     name VARCHAR(255) NOT NULL,                 
     age_group VARCHAR(50),                      
     capacity INTEGER,                           
     enrolled_count INTEGER DEFAULT 0,           
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: enrollments
 CREATE TABLE enrollments (                      
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     child_id UUID NOT NULL,                     
     school_id UUID NOT NULL,                    
     classroom_id UUID NOT NULL,                 
     status VARCHAR(50),                         
     application_status JSONB,                   
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: form_templates
 CREATE TABLE form_templates (                   
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     form_name VARCHAR(255) NOT NULL,            
     form_type VARCHAR(100),                     
     fillout_form_id VARCHAR(255),               
     due_date DATE,                              
     status VARCHAR(50),                         
     is_required BOOLEAN DEFAULT false,          
     display_order INTEGER,                      
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP                        
 );


-- Table: student_form_assignments
 CREATE TABLE student_form_assignments (                         
     id UUID DEFAULT uuid_generate_v4() NOT NULL,                
     school_id UUID NOT NULL,                                    
     enrollment_id UUID NOT NULL,                                
     child_id UUID NOT NULL,                                     
     form_template_id UUID NOT NULL,                             
     assignment_source VARCHAR(50),                              
     status VARCHAR(50) DEFAULT 'Not Started'::character varying,
     is_required BOOLEAN DEFAULT false,                          
     assigned_at TIMESTAMP DEFAULT now(),                        
     recent_form_submission_id UUID,                             
     approved_by UUID,                                           
     notes TEXT,                                                 
     approved_on TIMESTAMP,                                      
     is_active BOOLEAN DEFAULT true,                             
     created_at TIMESTAMP DEFAULT now(),                         
     updated_at TIMESTAMP,                                       
     recent_edit_link TEXT,                                      
     recent_pdf_link TEXT                                        
 );


-- Table: form_submissions
 CREATE TABLE form_submissions (                 
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     enrollment_id UUID NOT NULL,                
     student_form_assignment_id UUID NOT NULL,   
     form_template_id UUID NOT NULL,             
     fillout_submission_id VARCHAR(255) NOT NULL,
     form_data JSONB,                            
     metadata JSONB,                             
     submitted_at TIMESTAMP,                     
     processed_at TIMESTAMP,                     
     is_active BOOLEAN DEFAULT true,             
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP,                       
     edit_link TEXT,                             
     pdf_link TEXT                               
 );


-- Table: documents
 CREATE TABLE documents (                        
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     enrollment_id UUID NOT NULL,                
     school_id UUID NOT NULL,                    
     document_type VARCHAR(100),                 
     storage_path TEXT,                          
     file_name VARCHAR(255),                     
     uploaded_at TIMESTAMP DEFAULT now()         
 );


-- Table: class_transitions
 CREATE TABLE class_transitions (                
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     enrollment_id UUID NOT NULL,                
     child_id UUID NOT NULL,                     
     school_id UUID NOT NULL,                    
     from_classroom_id UUID NOT NULL,            
     to_classroom_id UUID NOT NULL,              
     changed_by UUID,                            
     reason TEXT,                                
     transitioned_at TIMESTAMP DEFAULT now(),    
     created_at TIMESTAMP DEFAULT now(),         
     is_active BOOLEAN DEFAULT true              
 );


-- Table: class_form_overrides
 CREATE TABLE class_form_overrides (             
     id UUID DEFAULT uuid_generate_v4() NOT NULL,
     school_id UUID NOT NULL,                    
     classroom_id UUID NOT NULL,                 
     form_template_id UUID NOT NULL,             
     action VARCHAR(50),                         
     is_required BOOLEAN,                        
     created_at TIMESTAMP DEFAULT now(),         
     updated_at TIMESTAMP,                       
     is_active BOOLEAN DEFAULT true              
 );


-- =============================================
-- PRIMARY KEYS
-- =============================================
 ALTER TABLE children ADD CONSTRAINT children_pkey PRIMARY KEY (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_pkey PRIMARY KEY (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_pkey PRIMARY KEY (id);
 ALTER TABLE classrooms ADD CONSTRAINT classrooms_pkey PRIMARY KEY (id);
 ALTER TABLE documents ADD CONSTRAINT documents_pkey PRIMARY KEY (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_pkey PRIMARY KEY (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_pkey PRIMARY KEY (id);
 ALTER TABLE form_templates ADD CONSTRAINT form_templates_pkey PRIMARY KEY (id);
 ALTER TABLE schools ADD CONSTRAINT schools_pkey PRIMARY KEY (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_pkey PRIMARY KEY (id);
 ALTER TABLE users ADD CONSTRAINT users_pkey PRIMARY KEY (id, id);


-- =============================================
-- UNIQUE CONSTRAINTS
-- =============================================
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_fillout_submission_id_key UNIQUE (fillout_submission_id);
 ALTER TABLE schools ADD CONSTRAINT schools_name_key UNIQUE (name);
 ALTER TABLE schools ADD CONSTRAINT schools_subdomain_key UNIQUE (subdomain);
 ALTER TABLE users ADD CONSTRAINT unique_email_per_school UNIQUE (school_id, email);


-- =============================================
-- FOREIGN KEYS
-- =============================================
 ALTER TABLE children ADD CONSTRAINT children_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES users (id);
 ALTER TABLE children ADD CONSTRAINT children_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE children ADD CONSTRAINT children_secondary_parent_id_fkey FOREIGN KEY (secondary_parent_id) REFERENCES users (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_classroom_id_fkey FOREIGN KEY (classroom_id) REFERENCES classrooms (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_form_template_id_fkey FOREIGN KEY (form_template_id) REFERENCES form_templates (id);
 ALTER TABLE class_form_overrides ADD CONSTRAINT class_form_overrides_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_changed_by_fkey FOREIGN KEY (changed_by) REFERENCES users (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_child_id_fkey FOREIGN KEY (child_id) REFERENCES children (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_from_classroom_id_fkey FOREIGN KEY (from_classroom_id) REFERENCES classrooms (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE class_transitions ADD CONSTRAINT class_transitions_to_classroom_id_fkey FOREIGN KEY (to_classroom_id) REFERENCES classrooms (id);
 ALTER TABLE classrooms ADD CONSTRAINT classrooms_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE documents ADD CONSTRAINT documents_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE documents ADD CONSTRAINT documents_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_child_id_fkey FOREIGN KEY (child_id) REFERENCES children (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_classroom_id_fkey FOREIGN KEY (classroom_id) REFERENCES classrooms (id);
 ALTER TABLE enrollments ADD CONSTRAINT enrollments_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_form_template_id_fkey FOREIGN KEY (form_template_id) REFERENCES form_templates (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE form_submissions ADD CONSTRAINT form_submissions_student_form_assignment_id_fkey FOREIGN KEY (student_form_assignment_id) REFERENCES student_form_assignments (id);
 ALTER TABLE form_templates ADD CONSTRAINT form_templates_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_child_id_fkey FOREIGN KEY (child_id) REFERENCES children (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_enrollment_id_fkey FOREIGN KEY (enrollment_id) REFERENCES enrollments (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_form_template_id_fkey FOREIGN KEY (form_template_id) REFERENCES form_templates (id);
 ALTER TABLE student_form_assignments ADD CONSTRAINT student_form_assignments_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);
 ALTER TABLE users ADD CONSTRAINT users_created_by_fkey FOREIGN KEY (created_by) REFERENCES users (id);
 ALTER TABLE users ADD CONSTRAINT users_school_id_fkey FOREIGN KEY (school_id) REFERENCES schools (id);


-- =============================================
-- FUNCTIONS
-- =============================================
                                                                                                                             
 -- Function: check_enrollment_completion                                                                                    
 CREATE OR REPLACE FUNCTION public.check_enrollment_completion(p_enrollment_id uuid)                                         
  RETURNS void                                                                                                               
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     total_forms INTEGER;                                                                                                    
     approved_forms INTEGER;                                                                                                 
     all_approved BOOLEAN;                                                                                                   
 BEGIN                                                                                                                       
     -- Count total active form assignments for this enrollment                                                              
     SELECT COUNT(*)                                                                                                         
     INTO total_forms                                                                                                        
     FROM student_form_assignments                                                                                           
     WHERE enrollment_id = p_enrollment_id                                                                                   
       AND is_active = true;                                                                                                 
                                                                                                                             
     -- Count how many forms are approved                                                                                    
     -- You can adjust the status check based on your business logic                                                         
     -- Options: 'approved', 'completed', or both                                                                            
     SELECT COUNT(*)                                                                                                         
     INTO approved_forms                                                                                                     
     FROM student_form_assignments                                                                                           
     WHERE enrollment_id = p_enrollment_id                                                                                   
       AND is_active = true                                                                                                  
       AND status IN ('approved', 'completed');  -- Adjust as needed                                                         
                                                                                                                             
     -- Determine if all forms are approved                                                                                  
     all_approved := (total_forms > 0 AND total_forms = approved_forms);                                                     
                                                                                                                             
     -- Update enrollment status based on completion                                                                         
     IF all_approved THEN                                                                                                    
         UPDATE enrollments                                                                                                  
         SET                                                                                                                 
             status = 'completed',                                                                                           
             updated_at = NOW()                                                                                              
         WHERE id = p_enrollment_id                                                                                          
           AND status != 'completed';  -- Only update if not already completed                                               
     ELSE                                                                                                                    
         -- Optional: Set back to 'incomplete' or 'pending' if not all approved                                              
         -- Uncomment the following if you want to auto-revert status                                                        
         /*                                                                                                                  
         UPDATE enrollments                                                                                                  
         SET                                                                                                                 
             status = 'incomplete',                                                                                          
             updated_at = NOW()                                                                                              
         WHERE id = p_enrollment_id                                                                                          
           AND status = 'completed';  -- Only downgrade if it was completed                                                  
         */                                                                                                                  
     END IF;                                                                                                                 
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: handle_new_auth_user                                                                                           
 CREATE OR REPLACE FUNCTION public.handle_new_auth_user()                                                                    
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
  SECURITY DEFINER                                                                                                           
 AS $function$                                                                                                               
 BEGIN                                                                                                                       
     -- Insert new user into public.users table using auth user data                                                         
     INSERT INTO public.users (                                                                                              
         id,                                                                                                                 
         school_id,                                                                                                          
         first_name,                                                                                                         
         last_name,                                                                                                          
         email,                                                                                                              
         role,                                                                                                               
         is_verified,                                                                                                        
         created_at,                                                                                                         
         metadata,                                                                                                           
         is_active                                                                                                           
     ) VALUES (                                                                                                              
         NEW.id,                                                                                                             
         COALESCE(                                                                                                           
             (NEW.raw_user_meta_data->>'school_id')::UUID,                                                                   
             NULL                                                                                                            
         ),                                                                                                                  
         COALESCE(                                                                                                           
             NEW.raw_user_meta_data->>'first_name',                                                                          
             ''                                                                                                              
         ),                                                                                                                  
         COALESCE(                                                                                                           
             NEW.raw_user_meta_data->>'last_name',                                                                           
             ''                                                                                                              
         ),                                                                                                                  
         NEW.email,                                                                                                          
         COALESCE(                                                                                                           
             NEW.raw_user_meta_data->>'role',                                                                                
             'Parent'                                                                                                        
         ),                                                                                                                  
         -- Respect is_verified from metadata if provided, otherwise default based on role                                   
         COALESCE(                                                                                                           
             (NEW.raw_user_meta_data->>'is_verified')::boolean,                                                              
             CASE                                                                                                            
                 WHEN COALESCE(NEW.raw_user_meta_data->>'role', 'Parent') IN ('Parent', 'primary-parent', 'secondary-parent')
                 THEN true                                                                                                   
                 ELSE false                                                                                                  
             END                                                                                                             
         ),                                                                                                                  
         NOW(),                                                                                                              
         NEW.raw_user_meta_data,                                                                                             
         true                                                                                                                
     );                                                                                                                      
                                                                                                                             
     RETURN NEW;                                                                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: set_is_verified_based_on_role                                                                                  
 CREATE OR REPLACE FUNCTION public.set_is_verified_based_on_role()                                                           
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 BEGIN                                                                                                                       
     -- Set is_verified based on role                                                                                        
     IF NEW.is_verified IS NULL THEN                                                                                         
         IF NEW.role = 'Parent' OR NEW.role = 'primary-parent' OR NEW.role = 'secondary-parent' THEN                         
             NEW.is_verified := true;                                                                                        
         ELSE                                                                                                                
             NEW.is_verified := false;                                                                                       
         END IF;                                                                                                             
     END IF;                                                                                                                 
     RETURN NEW;                                                                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: sync_all_enrollment_statuses                                                                                   
 CREATE OR REPLACE FUNCTION public.sync_all_enrollment_statuses()                                                            
  RETURNS void                                                                                                               
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     enrollment_record RECORD;                                                                                               
 BEGIN                                                                                                                       
     -- Loop through all enrollments and sync their statuses                                                                 
     FOR enrollment_record IN                                                                                                
         SELECT DISTINCT id FROM enrollments WHERE is_active = true                                                          
     LOOP                                                                                                                    
         -- Build and update application_status for each enrollment                                                          
         UPDATE enrollments e                                                                                                
         SET application_status = (                                                                                          
             SELECT COALESCE(                                                                                                
                 jsonb_object_agg(ft.form_name, sfa.status),                                                                 
                 '{}'::jsonb                                                                                                 
             )                                                                                                               
             FROM student_form_assignments sfa                                                                               
             JOIN form_templates ft ON ft.id = sfa.form_template_id                                                          
             WHERE sfa.enrollment_id = enrollment_record.id                                                                  
               AND sfa.is_active = true                                                                                      
         ),                                                                                                                  
         updated_at = NOW()                                                                                                  
         WHERE e.id = enrollment_record.id;                                                                                  
                                                                                                                             
         -- Check completion status                                                                                          
         PERFORM check_enrollment_completion(enrollment_record.id);                                                          
     END LOOP;                                                                                                               
                                                                                                                             
     RAISE NOTICE 'Synced all enrollment statuses successfully';                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: sync_enrollment_form_status                                                                                    
 CREATE OR REPLACE FUNCTION public.sync_enrollment_form_status()                                                             
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     target_enrollment_id UUID;                                                                                              
     new_application_status JSONB;                                                                                           
 BEGIN                                                                                                                       
     -- Determine which enrollment_id to update                                                                              
     -- Handle INSERT/UPDATE (use NEW) and DELETE (use OLD)                                                                  
     IF TG_OP = 'DELETE' THEN                                                                                                
         target_enrollment_id := OLD.enrollment_id;                                                                          
     ELSE                                                                                                                    
         target_enrollment_id := NEW.enrollment_id;                                                                          
     END IF;                                                                                                                 
                                                                                                                             
     -- Build the application_status JSONB by aggregating all form assignments                                               
     -- Join with form_templates to get the form_name                                                                        
     SELECT COALESCE(                                                                                                        
         jsonb_object_agg(                                                                                                   
             ft.form_name,  -- Key: form name from form_templates                                                            
             sfa.status     -- Value: status from student_form_assignments                                                   
         ),                                                                                                                  
         '{}'::jsonb  -- Empty JSONB if no forms found                                                                       
     )                                                                                                                       
     INTO new_application_status                                                                                             
     FROM student_form_assignments sfa                                                                                       
     JOIN form_templates ft ON ft.id = sfa.form_template_id                                                                  
     WHERE sfa.enrollment_id = target_enrollment_id                                                                          
       AND sfa.is_active = true;  -- Only include active assignments                                                         
                                                                                                                             
     -- Update the enrollments table with the new application_status                                                         
     UPDATE enrollments                                                                                                      
     SET                                                                                                                     
         application_status = new_application_status,                                                                        
         updated_at = NOW()                                                                                                  
     WHERE id = target_enrollment_id;                                                                                        
                                                                                                                             
     -- After updating application_status, check if enrollment should be completed                                           
     PERFORM check_enrollment_completion(target_enrollment_id);                                                              
                                                                                                                             
     -- Return appropriate value based on operation                                                                          
     IF TG_OP = 'DELETE' THEN                                                                                                
         RETURN OLD;                                                                                                         
     ELSE                                                                                                                    
         RETURN NEW;                                                                                                         
     END IF;                                                                                                                 
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: sync_existing_auth_users                                                                                       
 CREATE OR REPLACE FUNCTION public.sync_existing_auth_users()                                                                
  RETURNS void                                                                                                               
  LANGUAGE plpgsql                                                                                                           
  SECURITY DEFINER                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     auth_user RECORD;                                                                                                       
 BEGIN                                                                                                                       
     FOR auth_user IN                                                                                                        
         SELECT * FROM auth.users                                                                                            
         WHERE id NOT IN (SELECT id FROM public.users)                                                                       
     LOOP                                                                                                                    
         INSERT INTO public.users (                                                                                          
             id,                                                                                                             
             school_id,                                                                                                      
             first_name,                                                                                                     
             last_name,                                                                                                      
             email,                                                                                                          
             role,                                                                                                           
             is_verified,                                                                                                    
             created_at,                                                                                                     
             metadata,                                                                                                       
             is_active                                                                                                       
         ) VALUES (                                                                                                          
             auth_user.id,                                                                                                   
             COALESCE(                                                                                                       
                 (auth_user.raw_user_meta_data->>'school_id')::UUID,                                                         
                 NULL                                                                                                        
             ),                                                                                                              
             COALESCE(                                                                                                       
                 auth_user.raw_user_meta_data->>'first_name',                                                                
                 ''                                                                                                          
             ),                                                                                                              
             COALESCE(                                                                                                       
                 auth_user.raw_user_meta_data->>'last_name',                                                                 
                 ''                                                                                                          
             ),                                                                                                              
             auth_user.email,                                                                                                
             COALESCE(                                                                                                       
                 auth_user.raw_user_meta_data->>'role',                                                                      
                 'Parent'                                                                                                    
             ),                                                                                                              
             COALESCE(auth_user.email_confirmed_at IS NOT NULL, false),                                                      
             auth_user.created_at,                                                                                           
             auth_user.raw_user_meta_data,                                                                                   
             true                                                                                                            
         ) ON CONFLICT (id) DO NOTHING;                                                                                      
     END LOOP;                                                                                                               
 END;                                                                                                                        
 $function$                                                                                                                  
 
                                                                                                                             
 -- Function: track_classroom_transition                                                                                     
 CREATE OR REPLACE FUNCTION public.track_classroom_transition()                                                              
  RETURNS trigger                                                                                                            
  LANGUAGE plpgsql                                                                                                           
 AS $function$                                                                                                               
 DECLARE                                                                                                                     
     current_user_id UUID;                                                                                                   
     recent_transition_count INTEGER;                                                                                        
 BEGIN                                                                                                                       
     -- Only track if classroom actually changed                                                                             
     IF OLD.classroom_id IS DISTINCT FROM NEW.classroom_id THEN                                                              
                                                                                                                             
         -- Check if a transition was just updated in the last 2 seconds (edit sync scenario)                                
         SELECT COUNT(*) INTO recent_transition_count                                                                        
         FROM class_transitions                                                                                              
         WHERE enrollment_id = NEW.id                                                                                        
         AND from_classroom_id = OLD.classroom_id                                                                            
         AND to_classroom_id = NEW.classroom_id                                                                              
         AND created_at > NOW() - INTERVAL '2 seconds';                                                                      
                                                                                                                             
         -- Skip if duplicate found                                                                                          
         IF recent_transition_count > 0 THEN                                                                                 
             RETURN NEW;                                                                                                     
         END IF;                                                                                                             
                                                                                                                             
         -- Always create transition record (removed form submission check)                                                  
         BEGIN                                                                                                               
             current_user_id := current_setting('app.current_user_id', true)::UUID;                                          
         EXCEPTION WHEN OTHERS THEN                                                                                          
             current_user_id := NULL;                                                                                        
         END;                                                                                                                
                                                                                                                             
         INSERT INTO class_transitions (                                                                                     
             enrollment_id,                                                                                                  
             child_id,                                                                                                       
             school_id,                                                                                                      
             from_classroom_id,                                                                                              
             to_classroom_id,                                                                                                
             changed_by,                                                                                                     
             transitioned_at                                                                                                 
         ) VALUES (                                                                                                          
             NEW.id,                                                                                                         
             NEW.child_id,                                                                                                   
             NEW.school_id,                                                                                                  
             OLD.classroom_id,                                                                                               
             NEW.classroom_id,                                                                                               
             current_user_id,                                                                                                
             NOW()                                                                                                           
         );                                                                                                                  
     END IF;                                                                                                                 
     RETURN NEW;                                                                                                             
 END;                                                                                                                        
 $function$                                                                                                                  
 


-- =============================================
-- TRIGGERS
-- =============================================
                                                                                                                                                         
 -- Trigger on table: enrollments                                                                                                                        
 CREATE TRIGGER trigger_track_classroom_transition AFTER UPDATE ON enrollments FOR EACH ROW EXECUTE FUNCTION track_classroom_transition();
                                                                                                                                                         
 -- Trigger on table: student_form_assignments                                                                                                           
 CREATE TRIGGER trigger_sync_enrollment_form_status AFTER INSERT ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION sync_enrollment_form_status();
                                                                                                                                                         
 -- Trigger on table: student_form_assignments                                                                                                           
 CREATE TRIGGER trigger_sync_enrollment_form_status AFTER DELETE ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION sync_enrollment_form_status();
                                                                                                                                                         
 -- Trigger on table: student_form_assignments                                                                                                           
 CREATE TRIGGER trigger_sync_enrollment_form_status AFTER UPDATE ON student_form_assignments FOR EACH ROW EXECUTE FUNCTION sync_enrollment_form_status();
                                                                                                                                                         
 -- Trigger on table: users                                                                                                                              
 CREATE TRIGGER set_is_verified_on_insert BEFORE INSERT ON users FOR EACH ROW EXECUTE FUNCTION set_is_verified_based_on_role();

