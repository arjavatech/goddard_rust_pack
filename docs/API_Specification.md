# The Goddard School Enrollment Management System
## REST API Specification v1.0.0

### Executive Summary
This document defines the comprehensive REST API specification for The Goddard School Enrollment Management System, a multi-tenant SaaS platform designed to digitize the child enrollment process for Goddard School franchises. The API follows Google Cloud API design principles and is optimized for implementation in Rust with AWS Lambda.

---

## Table of Contents
1. [Overview](#overview)
2. [Authentication & Authorization](#authentication--authorization)
3. [Core Resource APIs](#core-resource-apis)
4. [Form Management APIs](#form-management-apis)
5. [Communication & Document APIs](#communication--document-apis)
6. [Administrative APIs](#administrative-apis)
7. [Error Handling](#error-handling)
8. [Implementation Guide](#implementation-guide)
9. [OpenAPI Specification](#openapi-specification)

---

## 1. Overview

### 1.1 API Design Principles

Following Google Cloud API design best practices:

- **Resource-Oriented Design**: APIs organized around business resources
- **Outside-In Design**: Focused on user experience and customer needs
- **Standard HTTP Methods**: Consistent use of GET, POST, PUT, PATCH, DELETE
- **Multi-Tenant Security**: Row-Level Security (RLS) with school context isolation
- **Simple & Intuitive**: Minimize complexity while maximizing functionality
- **Scalable Architecture**: Support for 500+ concurrent users across multiple schools

### 1.2 Base Configuration

```
Base URL: https://api.goddard.com/v1
Content-Type: application/json
Authentication: Supabase Auth (Bearer JWT tokens)
Multi-Tenancy: School context via user profiles
Rate Limiting: 1000 requests/minute per school, 100/minute per user
```

### 1.3 Multi-Tenant Architecture

The API supports multi-tenancy through:
- **School Context Resolution**: Extracted from user profiles linked to Supabase Auth
- **Data Isolation**: PostgreSQL RLS policies with auth.uid() ensure complete data separation
- **Resource Scoping**: All resources automatically scoped to authenticated school via user profile
- **Authorization**: Role-based access control using Supabase Auth and custom user roles

### 1.4 Standard Response Structure

```json
{
  "data": {},           // Response payload
  "meta": {             // Metadata (pagination, counts, etc.)
    "page": 1,
    "per_page": 20,
    "total": 100,
    "total_pages": 5
  },
  "links": {            // Navigation links for pagination
    "self": "/api/v1/children?page=1",
    "next": "/api/v1/children?page=2",
    "prev": null,
    "first": "/api/v1/children?page=1",
    "last": "/api/v1/children?page=5"
  }
}
```

### 1.5 Error Response Structure (RFC 7807)

```json
{
  "type": "https://api.goddard.com/errors/validation-error",
  "title": "Validation Error",
  "status": 400,
  "detail": "The email address format is invalid",
  "instance": "/api/v1/users",
  "school_id": "550e8400-e29b-41d4-a716-446655440000",
  "errors": [
    {
      "field": "email",
      "code": "invalid_format",
      "message": "Email must be a valid email address"
    }
  ]
}
```

---

## 2. Authentication & Authorization

### 2.1 Supabase Auth Integration

The Goddard School system uses **Supabase Auth** for all authentication and user management. This provides enterprise-grade security with built-in features like rate limiting, bot protection, and multiple authentication methods.

#### 2.1.1 Authentication Methods Supported

- **📧 Email & Password**: Traditional email/password authentication
- **🔗 Magic Links**: Passwordless email authentication
- **📱 Phone Authentication**: SMS-based login with OTP
- **🌐 OAuth Providers**: Google, Apple, GitHub, and 20+ other providers
- **🚪 Single Sign-On (SSO)**: Enterprise SSO integration
- **👤 Anonymous Sign-in**: Guest access for certain features

#### 2.1.2 Client-Side Authentication Flow

```javascript
// Initialize Supabase client
import { createClient } from '@supabase/supabase-js'

const supabase = createClient(
  'https://your-project.supabase.co',
  'your-anon-key'
)

// Email & Password Sign Up with School Context
const { data, error } = await supabase.auth.signUp({
  email: 'parent@example.com',
  password: 'secure_password',
  options: {
    data: {
      first_name: 'John',
      last_name: 'Doe',
      school_id: '550e8400-e29b-41d4-a716-446655440000',
      role: 'parent'
    }
  }
})

// Sign In
const { data, error } = await supabase.auth.signInWithPassword({
  email: 'parent@example.com',
  password: 'secure_password'
})

// Magic Link Authentication
const { data, error } = await supabase.auth.signInWithOtp({
  email: 'parent@example.com',
  options: {
    emailRedirectTo: 'https://brookside.goddard.com/auth/callback'
  }
})

// OAuth Sign In (Google)
const { data, error } = await supabase.auth.signInWithOAuth({
  provider: 'google',
  options: {
    redirectTo: 'https://brookside.goddard.com/auth/callback',
    queryParams: {
      school_id: '550e8400-e29b-41d4-a716-446655440000'
    }
  }
})

// Get Current Session
const { data: { session } } = await supabase.auth.getSession()

// Sign Out
const { error } = await supabase.auth.signOut()
```

### 2.2 User Profile Management

Since Supabase Auth handles core authentication, we use a **profiles table** for extended user data and school context.

#### 2.2.1 Database Schema Pattern

```sql
-- Profiles table linked to auth.users
CREATE TABLE profiles (
  id UUID REFERENCES auth.users(id) ON DELETE CASCADE PRIMARY KEY,
  school_id UUID REFERENCES schools(id) NOT NULL,
  role VARCHAR(20) NOT NULL CHECK (role IN ('parent', 'teacher', 'admin', 'super_admin')),
  first_name VARCHAR(100) NOT NULL,
  last_name VARCHAR(100) NOT NULL,
  phone VARCHAR(20),
  is_active BOOLEAN DEFAULT TRUE,
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW(),
  
  -- Indexes for performance
  CONSTRAINT unique_user_per_school UNIQUE(id, school_id)
);

-- RLS Policies using Supabase Auth
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;

-- Users can view their own profile
CREATE POLICY "Users can view own profile" ON profiles
  FOR SELECT USING (auth.uid() = id);

-- Admins can view profiles in their school
CREATE POLICY "Admins can view school profiles" ON profiles
  FOR SELECT USING (
    EXISTS (
      SELECT 1 FROM profiles admin_profile
      WHERE admin_profile.id = auth.uid()
      AND admin_profile.school_id = profiles.school_id
      AND admin_profile.role IN ('admin', 'super_admin')
    )
  );
```

#### 2.2.2 Profile Creation Trigger

```sql
-- Automatically create profile when user signs up
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO public.profiles (id, school_id, role, first_name, last_name)
  VALUES (
    NEW.id,
    (NEW.raw_user_meta_data->>'school_id')::UUID,
    COALESCE(NEW.raw_user_meta_data->>'role', 'parent'),
    NEW.raw_user_meta_data->>'first_name',
    NEW.raw_user_meta_data->>'last_name'
  );
  RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Trigger on auth.users
CREATE TRIGGER on_auth_user_created
  AFTER INSERT ON auth.users
  FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();
```

### 2.3 Server-Side Authentication

For server-side API endpoints, verify Supabase JWT tokens and extract user context.

#### 2.3.1 Token Verification (Rust)

```rust
use supabase_auth::verify_jwt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub role: String,
    pub email: String,
}

pub async fn verify_supabase_token(token: &str) -> Result<AuthContext, AuthError> {
    // Verify JWT token with Supabase
    let claims = verify_jwt(token, &env::var("SUPABASE_JWT_SECRET")?).await?;
    
    // Extract user ID from JWT
    let user_id = Uuid::parse_str(&claims.sub)?;
    
    // Get user profile from database
    let profile = sqlx::query!(
        "SELECT school_id, role, first_name, last_name 
         FROM profiles 
         WHERE id = $1 AND is_active = true",
        user_id
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or(AuthError::UserNotFound)?;
    
    Ok(AuthContext {
        user_id,
        school_id: profile.school_id,
        role: profile.role,
        email: claims.email.unwrap_or_default(),
    })
}
```

### 2.4 School Context Resolution

#### 2.4.1 Get User's School Context

```http
GET /api/v1/me/school
Authorization: Bearer {supabase_jwt_token}

Response:
{
  "data": {
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "email": "parent@example.com",
      "role": "parent",
      "first_name": "John",
      "last_name": "Doe"
    },
    "school": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Brookside Goddard School",
      "subdomain": "brookside",
      "settings": {
        "primary_color": "#2563eb",
        "logo_url": "https://cdn.goddard.com/schools/brookside/logo.png"
      }
    }
  }
}
```

#### 2.4.2 Update User Profile

```http
PATCH /api/v1/me/profile
Authorization: Bearer {supabase_jwt_token}
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe-Smith",
  "phone": "(555) 123-9999"
}

Response:
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "first_name": "John",
    "last_name": "Doe-Smith",
    "phone": "(555) 123-9999",
    "updated_at": "2024-01-16T10:30:00Z"
  }
}
```

### 2.5 Role-Based Access Control

| Role | Permissions | Scope | RLS Pattern |
|------|-------------|-------|-------------|
| **parent** | View/edit own children, complete forms | Own children only | `auth.uid() = parent_id` |
| **teacher** | View assigned classroom students | Assigned classrooms | `EXISTS (classroom assignment)` |
| **admin** | Full school management | Entire school | `school_id = user.school_id` |
| **super_admin** | System administration | All schools | No restrictions |

### 2.6 Authentication State Management

#### 2.6.1 Session Handling

```javascript
// Listen for auth state changes
supabase.auth.onAuthStateChange((event, session) => {
  if (event === 'SIGNED_IN') {
    // User signed in - redirect to dashboard
    window.location.href = '/dashboard'
  }
  
  if (event === 'SIGNED_OUT') {
    // User signed out - redirect to login
    window.location.href = '/login'
  }
  
  if (event === 'TOKEN_REFRESHED') {
    // Token refreshed automatically
    console.log('Token refreshed:', session.access_token)
  }
})

// Check if user is authenticated
const { data: { user } } = await supabase.auth.getUser()
if (!user) {
  // Redirect to login
  window.location.href = '/login'
}
```

#### 2.6.2 Password Reset Flow

```javascript
// Request password reset
const { data, error } = await supabase.auth.resetPasswordForEmail(
  'parent@example.com',
  {
    redirectTo: 'https://brookside.goddard.com/reset-password'
  }
)

// Handle password reset (on reset page)
const { data, error } = await supabase.auth.updateUser({
  password: 'new_secure_password'
})
```

### 2.7 Security Features

#### 2.7.1 Built-in Security
- **Rate Limiting**: Automatic protection against brute force attacks
- **Bot Protection**: CAPTCHA integration for suspicious activity  
- **Email Verification**: Mandatory email verification for new accounts
- **JWT Security**: Automatic token rotation and secure storage
- **Session Management**: Configurable session timeouts and persistence

#### 2.7.2 Multi-Factor Authentication

```javascript
// Enable MFA for user
const { data, error } = await supabase.auth.mfa.enroll({
  factorType: 'totp',
  friendlyName: 'Goddard School App'
})

// Verify MFA challenge
const { data, error } = await supabase.auth.mfa.challengeAndVerify({
  factorId: 'factor-id',
  code: '123456'
})
```

### 2.8 Error Handling

Comprehensive error handling patterns for Supabase Auth integration.

#### 2.8.1 Client-Side Error Handling

```typescript
interface SupabaseAuthError {
  message: string;
  status?: number;
  code?: string;
}

interface ProfileError {
  message: string;
  details?: string;
}

// Comprehensive auth error handling
const handleSupabaseAuthError = (error: SupabaseAuthError): string => {
  // Authentication errors
  if (error.message.includes('Invalid login credentials')) {
    return 'Invalid email or password. Please check your credentials.'
  }
  
  if (error.message.includes('Email not confirmed')) {
    return 'Please check your email and click the confirmation link before signing in.'
  }
  
  if (error.message.includes('Password should be at least')) {
    return 'Password must be at least 6 characters long.'
  }
  
  if (error.message.includes('User not found')) {
    return 'No account found with this email address.'
  }
  
  if (error.message.includes('Email address already in use')) {
    return 'An account with this email address already exists.'
  }
  
  // Session and token errors
  if (error.message.includes('JWT expired')) {
    return 'Your session has expired. Please sign in again.'
  }
  
  if (error.message.includes('JWT is invalid')) {
    return 'Invalid authentication token. Please sign in again.'
  }
  
  // Rate limiting errors
  if (error.message.includes('too many requests')) {
    return 'Too many attempts. Please wait a moment before trying again.'
  }
  
  // Network and service errors
  if (error.message.includes('network') || error.status === 0) {
    return 'Network error. Please check your connection and try again.'
  }
  
  if (error.status && error.status >= 500) {
    return 'Service temporarily unavailable. Please try again later.'
  }
  
  // Fallback
  return 'Authentication failed. Please try again or contact support if the problem persists.'
}

// Profile-related error handling
const handleProfileError = (error: ProfileError): string => {
  if (error.message.includes('school_id')) {
    return 'Invalid school assignment. Please contact your administrator.'
  }
  
  if (error.message.includes('profile not found')) {
    return 'User profile not found. Please complete your registration.'
  }
  
  if (error.message.includes('permission denied')) {
    return 'You do not have permission to access this resource.'
  }
  
  return error.message || 'Profile error occurred. Please try again.'
}

// React hook for error handling
const useAuthErrorHandler = () => {
  const handleError = useCallback((error: any) => {
    if (error?.name === 'AuthError') {
      toast.error(handleSupabaseAuthError(error))
    } else if (error?.message?.includes('profile')) {
      toast.error(handleProfileError(error))
    } else {
      toast.error('An unexpected error occurred. Please try again.')
    }
  }, [])

  return { handleError }
}
```

#### 2.8.2 Server-Side Error Handling (Rust)

```rust
// Enhanced auth errors for Supabase integration
use axum::http::StatusCode;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SupabaseAuthError {
    #[error("Invalid JWT token: {0}")]
    InvalidToken(String),
    
    #[error("JWT token expired")]
    TokenExpired,
    
    #[error("Profile not found for user: {0}")]
    ProfileNotFound(String),
    
    #[error("School context missing or invalid")]
    InvalidSchoolContext,
    
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    
    #[error("Supabase service error: {0}")]
    ServiceError(String),
    
    #[error("Database profile query failed: {0}")]
    ProfileQueryFailed(String),
}

impl SupabaseAuthError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            SupabaseAuthError::InvalidToken(_) 
            | SupabaseAuthError::TokenExpired => StatusCode::UNAUTHORIZED,
            
            SupabaseAuthError::InsufficientPermissions(_) => StatusCode::FORBIDDEN,
            
            SupabaseAuthError::ProfileNotFound(_) 
            | SupabaseAuthError::InvalidSchoolContext => StatusCode::NOT_FOUND,
            
            SupabaseAuthError::ServiceError(_) 
            | SupabaseAuthError::ProfileQueryFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    
    pub fn error_type(&self) -> &'static str {
        match self {
            SupabaseAuthError::InvalidToken(_) 
            | SupabaseAuthError::TokenExpired => "authentication_error",
            
            SupabaseAuthError::InsufficientPermissions(_) => "authorization_error",
            
            SupabaseAuthError::ProfileNotFound(_) 
            | SupabaseAuthError::InvalidSchoolContext => "profile_error",
            
            SupabaseAuthError::ServiceError(_) 
            | SupabaseAuthError::ProfileQueryFailed(_) => "service_error",
        }
    }
}

impl IntoResponse for SupabaseAuthError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_type = self.error_type();
        
        let error_response = json!({
            "type": format!("https://api.goddard.com/errors/{}", error_type),
            "title": status.canonical_reason().unwrap_or("Authentication Error"),
            "status": status.as_u16(),
            "detail": self.to_string(),
            "instance": "/auth",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        (status, Json(error_response)).into_response()
    }
}

// Enhanced middleware error handling
pub async fn supabase_auth_middleware(
    State(db): State<Database>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, SupabaseAuthError> {
    let token = extract_token(&headers)
        .map_err(|_| SupabaseAuthError::InvalidToken("Missing Authorization header".to_string()))?;
    
    let claims = verify_supabase_jwt(&token)
        .map_err(|e| match e {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => SupabaseAuthError::TokenExpired,
            _ => SupabaseAuthError::InvalidToken(e.to_string()),
        })?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| SupabaseAuthError::InvalidToken("Invalid user ID in token".to_string()))?;

    // Get user's profile and school context
    let profile = db.get_user_profile(user_id)
        .await
        .map_err(|e| SupabaseAuthError::ProfileQueryFailed(e.to_string()))?
        .ok_or_else(|| SupabaseAuthError::ProfileNotFound(user_id.to_string()))?;

    if !profile.is_active {
        return Err(SupabaseAuthError::InsufficientPermissions("Account is disabled".to_string()));
    }

    // Set RLS context
    db.set_school_context(profile.school_id)
        .await
        .map_err(|e| SupabaseAuthError::ServiceError(e.to_string()))?;

    let auth_context = AuthContext {
        user_id,
        email: claims.email,
        school_id: profile.school_id,
        role: profile.role,
    };

    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}
```

#### 2.8.3 Error Recovery Patterns

```typescript
// Automatic token refresh on expiration
const withAuthRetry = async <T>(operation: () => Promise<T>): Promise<T> => {
  try {
    return await operation()
  } catch (error) {
    if (error.message?.includes('JWT expired')) {
      // Attempt token refresh
      const { data, error: refreshError } = await supabase.auth.refreshSession()
      
      if (refreshError) {
        // Redirect to login if refresh fails
        redirectToLogin()
        throw new Error('Session expired. Please log in again.')
      }
      
      // Retry original operation with new token
      return await operation()
    }
    
    throw error
  }
}

// Usage example
const fetchUserData = () => withAuthRetry(async () => {
  const response = await fetch('/api/v1/profile', {
    headers: {
      'Authorization': `Bearer ${session?.access_token}`
    }
  })
  
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
  
  return response.json()
})
```

---

### 2.9 Supabase Setup & Configuration

Complete setup guide for integrating Supabase Auth with The Goddard School system.

#### 2.9.1 Supabase Project Configuration

**1. Create Supabase Project**
```bash
# Using Supabase CLI
npx supabase login
npx supabase projects create goddard-school
```

**2. Environment Variables**
```env
# .env.local (Next.js/React)
NEXT_PUBLIC_SUPABASE_URL=https://your-project.supabase.co
NEXT_PUBLIC_SUPABASE_ANON_KEY=your-anon-key
SUPABASE_SERVICE_ROLE_KEY=your-service-role-key

# Backend (.env for Rust)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_ROLE_KEY=your-service-role-key
SUPABASE_JWT_SECRET=your-jwt-secret
DATABASE_URL=postgresql://postgres:[password]@db.your-project.supabase.co:5432/postgres
```

#### 2.9.2 Database Migrations

**Migration 001: Create profiles table and RLS policies**
```sql
-- /migrations/001_create_profiles.sql

-- Create profiles table
CREATE TABLE public.profiles (
    id UUID REFERENCES auth.users(id) ON DELETE CASCADE PRIMARY KEY,
    school_id UUID REFERENCES schools(id) NOT NULL,
    role VARCHAR(20) NOT NULL CHECK (role IN ('parent', 'teacher', 'admin', 'super_admin')),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    phone VARCHAR(20),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Enable RLS
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;

-- Create RLS policies
CREATE POLICY "Users can view own profile" ON profiles
    FOR SELECT
    TO authenticated
    USING (auth.uid() = id);

CREATE POLICY "Users can update own profile" ON profiles
    FOR UPDATE
    TO authenticated
    USING (auth.uid() = id);

-- School isolation policy (used by service role)
CREATE POLICY "School isolation" ON profiles
    FOR ALL
    TO service_role
    USING (school_id = (current_setting('app.current_school_id', true))::uuid);

-- Create profile on user signup
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO public.profiles (
        id, 
        school_id, 
        role, 
        first_name, 
        last_name, 
        phone
    )
    VALUES (
        NEW.id,
        (NEW.raw_user_meta_data->>'school_id')::uuid,
        COALESCE(NEW.raw_user_meta_data->>'role', 'parent'),
        NEW.raw_user_meta_data->>'first_name',
        NEW.raw_user_meta_data->>'last_name',
        NEW.raw_user_meta_data->>'phone'
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Create trigger
DROP TRIGGER IF EXISTS on_auth_user_created ON auth.users;
CREATE TRIGGER on_auth_user_created
    AFTER INSERT ON auth.users
    FOR EACH ROW
    EXECUTE FUNCTION handle_new_user();
```

**Migration 002: Update existing tables for Supabase Auth**
```sql
-- /migrations/002_update_rls_policies.sql

-- Update all existing tables to use auth.uid()
-- Schools table
ALTER TABLE schools ENABLE ROW LEVEL SECURITY;
CREATE POLICY "School access based on profile" ON schools
    FOR ALL
    TO authenticated
    USING (
        id IN (
            SELECT school_id FROM profiles 
            WHERE id = auth.uid()
        )
    );

-- Children table
ALTER TABLE children ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Children access via school context" ON children
    FOR ALL
    TO authenticated
    USING (
        school_id IN (
            SELECT school_id FROM profiles 
            WHERE id = auth.uid()
        )
    );

-- Enrollments table
ALTER TABLE enrollments ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Enrollments access via school context" ON enrollments
    FOR ALL
    TO authenticated
    USING (
        school_id IN (
            SELECT school_id FROM profiles 
            WHERE id = auth.uid()
        )
    );

-- Form management tables
ALTER TABLE form_templates ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Form templates school isolation" ON form_templates
    FOR ALL
    TO authenticated
    USING (
        school_id IN (
            SELECT school_id FROM profiles 
            WHERE id = auth.uid()
        )
    );

-- Update other tables similarly...
-- (Continue for all multi-tenant tables)
```

#### 2.9.3 Client-Side Setup

**React/Next.js Supabase Client**
```typescript
// /lib/supabase.ts
import { createClientComponentClient } from '@supabase/auth-helpers-nextjs'
import { Database } from '@/types/database'

export const createClient = () => 
  createClientComponentClient<Database>()

// Alternative for static generation
import { createClient } from '@supabase/supabase-js'

const supabaseUrl = process.env.NEXT_PUBLIC_SUPABASE_URL!
const supabaseAnonKey = process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!

export const supabase = createClient(supabaseUrl, supabaseAnonKey)
```

**Authentication Provider Setup**
```typescript
// /contexts/AuthContext.tsx
'use client'
import { createContext, useContext, useEffect, useState } from 'react'
import { User } from '@supabase/supabase-js'
import { createClient } from '@/lib/supabase'

interface AuthContextType {
  user: User | null
  profile: UserProfile | null
  loading: boolean
  signUp: (email: string, password: string, metadata: any) => Promise<void>
  signIn: (email: string, password: string) => Promise<void>
  signOut: () => Promise<void>
}

const AuthContext = createContext<AuthContextType>({} as AuthContextType)

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [profile, setProfile] = useState<UserProfile | null>(null)
  const [loading, setLoading] = useState(true)
  const supabase = createClient()

  useEffect(() => {
    const getSession = async () => {
      const { data: { session } } = await supabase.auth.getSession()
      setUser(session?.user ?? null)
      
      if (session?.user) {
        await fetchProfile(session.user.id)
      }
      setLoading(false)
    }

    getSession()

    const { data: { subscription } } = supabase.auth.onAuthStateChange(
      async (event, session) => {
        setUser(session?.user ?? null)
        
        if (session?.user) {
          await fetchProfile(session.user.id)
        } else {
          setProfile(null)
        }
        setLoading(false)
      }
    )

    return () => subscription.unsubscribe()
  }, [])

  const fetchProfile = async (userId: string) => {
    const { data } = await supabase
      .from('profiles')
      .select('*, schools(*)')
      .eq('id', userId)
      .single()
    
    setProfile(data)
  }

  const signUp = async (email: string, password: string, metadata: any) => {
    const { error } = await supabase.auth.signUp({
      email,
      password,
      options: { data: metadata }
    })
    
    if (error) throw error
  }

  const signIn = async (email: string, password: string) => {
    const { error } = await supabase.auth.signInWithPassword({
      email,
      password
    })
    
    if (error) throw error
  }

  const signOut = async () => {
    const { error } = await supabase.auth.signOut()
    if (error) throw error
  }

  return (
    <AuthContext.Provider value={{
      user,
      profile,
      loading,
      signUp,
      signIn,
      signOut
    }}>
      {children}
    </AuthContext.Provider>
  )
}

export const useAuth = () => {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within AuthProvider')
  }
  return context
}
```

#### 2.9.4 Rust Backend Setup

**Cargo.toml Dependencies**
```toml
[dependencies]
# Supabase specific
supabase-auth = "0.2"
jsonwebtoken = "8.3"
serde_json = "1.0"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }
tokio = { version = "1.0", features = ["full"] }

# Web framework
axum = { version = "0.6", features = ["headers"] }
tower = "0.4"
tower-http = { version = "0.4", features = ["cors", "trace"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

**Database Connection with RLS Context**
```rust
// src/database.rs
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    // Set RLS context for multi-tenancy
    pub async fn set_school_context(
        &self, 
        school_id: Uuid
    ) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT set_config('app.current_school_id', $1, true)")
            .bind(school_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Get user's school context from profile
    pub async fn get_user_school_id(
        &self,
        user_id: Uuid
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            "SELECT school_id FROM profiles WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("school_id"))
    }
}
```

**Authentication Middleware (Updated)**
```rust
// src/auth/middleware.rs
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::database::Database;

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseClaims {
    pub sub: String,      // User ID
    pub email: String,
    pub aud: String,      // Audience
    pub exp: i64,         // Expiration
    pub iat: i64,         // Issued at
    pub iss: String,      // Issuer
    pub user_metadata: serde_json::Value,
    pub app_metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub email: String,
    pub school_id: Uuid,
    pub role: String,
}

pub async fn supabase_auth_middleware(
    State(db): State<Database>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_token(&headers)?;
    let claims = verify_supabase_jwt(&token)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get user's school context from database
    let school_id = db.get_user_school_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set RLS context for this request
    db.set_school_context(school_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let auth_context = AuthContext {
        user_id,
        email: claims.email,
        school_id,
        role: "parent".to_string(), // Get from profile if needed
    };

    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}

fn extract_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(auth_header.trim_start_matches("Bearer ").to_string())
}

fn verify_supabase_jwt(token: &str) -> Result<SupabaseClaims, StatusCode> {
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let key = DecodingKey::from_secret(jwt_secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    decode::<SupabaseClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}
```

#### 2.9.5 Development Workflow

**1. Local Development Setup**
```bash
# Start Supabase locally
npx supabase start

# Apply migrations
npx supabase db reset

# Generate types
npx supabase gen types typescript --local > types/database.ts
```

**2. Testing Authentication**
```typescript
// Example test for authentication flow
const testAuth = async () => {
  // Sign up new parent
  const { data, error } = await supabase.auth.signUp({
    email: 'parent@test.com',
    password: 'securepass123',
    options: {
      data: {
        first_name: 'John',
        last_name: 'Doe',
        school_id: 'school-uuid-here',
        role: 'parent'
      }
    }
  })

  if (error) {
    console.error('Signup failed:', error.message)
    return
  }

  console.log('User created:', data.user?.id)
  console.log('Profile will be created automatically via trigger')
}
```

**3. Production Deployment**
```bash
# Deploy to Supabase
npx supabase db push

# Update environment variables in production
# - Vercel/Netlify for frontend
# - AWS Lambda/Railway for backend
```

---

## 3. Core Resource APIs

### 3.1 Schools Management

Schools represent individual Goddard franchise locations in the multi-tenant system.

#### Get School Details
```http
GET /api/v1/school
Authorization: Bearer {jwt_token}

Response:
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Brookside Goddard School",
    "subdomain": "brookside",
    "settings": {
      "primary_color": "#2563eb",
      "secondary_color": "#64748b",
      "logo_url": "https://cdn.goddard.com/schools/brookside/logo.png",
      "contact_email": "info@brookside.goddard.com",
      "phone": "(555) 123-4567",
      "address": {
        "street": "123 Main St",
        "city": "Anytown",
        "state": "NY",
        "zip": "12345"
      },
      "enrollment_settings": {
        "auto_assign_forms": true,
        "require_photo_consent": true,
        "max_enrollment_days": 30
      }
    },
    "is_active": true,
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

#### Update School Settings (Admin Only)
```http
PUT /api/v1/school
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "settings": {
    "primary_color": "#1d4ed8",
    "logo_url": "https://cdn.goddard.com/schools/brookside/new-logo.png",
    "enrollment_settings": {
      "max_enrollment_days": 45
    }
  }
}
```

### 3.2 User Management

With Supabase Auth, user authentication is handled by Supabase, while we manage user profiles and school context through our `profiles` table. Admins can manage user profiles within their school.

#### List User Profiles
```http
GET /api/v1/profiles?role=parent&limit=20&page=1&sort=last_name
Authorization: Bearer {supabase_jwt_token}

Response:
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "email": "parent1@example.com",
      "role": "parent",
      "first_name": "John",
      "last_name": "Doe",
      "phone": "(555) 123-4567",
      "is_active": true,
      "school_id": "550e8400-e29b-41d4-a716-446655440000",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T09:30:00Z",
      "auth_user": {
        "email_confirmed_at": "2024-01-01T00:30:00Z",
        "last_sign_in_at": "2024-01-15T09:30:00Z"
      },
      "children_count": 2
    }
  ],
  "meta": {
    "page": 1,
    "per_page": 20,
    "total": 150,
    "total_pages": 8
  }
}
```

#### Invite New User (Admin Only)
Since user creation happens through Supabase Auth, admins send invitations that trigger the signup flow.

```http
POST /api/v1/invites
Authorization: Bearer {admin_supabase_jwt_token}
Content-Type: application/json

{
  "email": "newparent@example.com",
  "role": "parent",
  "first_name": "Jane",
  "last_name": "Smith",
  "phone": "(555) 987-6543",
  "send_welcome_email": true
}

Response:
{
  "data": {
    "invite_id": "invite-uuid",
    "email": "newparent@example.com",
    "role": "parent",
    "school_id": "550e8400-e29b-41d4-a716-446655440000",
    "invited_by": "admin-uuid",
    "invite_url": "https://brookside.goddard.com/signup?invite=invite-token",
    "expires_at": "2024-01-23T10:00:00Z",
    "created_at": "2024-01-16T10:00:00Z"
  }
}
```

#### User Signup Flow (Client-Side)
When a user receives an invitation, they complete signup through Supabase Auth:

```javascript
// On signup page with invite token
const { data, error } = await supabase.auth.signUp({
  email: inviteData.email,
  password: userPassword,
  options: {
    data: {
      first_name: inviteData.first_name,
      last_name: inviteData.last_name,
      school_id: inviteData.school_id,
      role: inviteData.role,
      invite_token: inviteData.token
    },
    emailRedirectTo: 'https://brookside.goddard.com/auth/callback'
  }
})
```

#### Get User Profile Details
```http
GET /api/v1/profiles/{user_id}
Authorization: Bearer {supabase_jwt_token}

# Parents can only access their own profile
# Admins can access any user profile in their school

Response:
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "parent@example.com",
    "role": "parent", 
    "first_name": "John",
    "last_name": "Doe",
    "phone": "(555) 123-4567",
    "is_active": true,
    "school": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Brookside Goddard School",
      "subdomain": "brookside"
    },
    "auth_user": {
      "email": "parent@example.com",
      "email_confirmed_at": "2024-01-01T00:30:00Z",
      "last_sign_in_at": "2024-01-15T09:30:00Z",
      "created_at": "2024-01-01T00:00:00Z"
    },
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-15T14:20:00Z"
  }
}
```

#### Update User Profile
```http
PATCH /api/v1/profiles/{user_id}
Authorization: Bearer {supabase_jwt_token}
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe-Smith", 
  "phone": "(555) 123-9999"
}

Response:
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "first_name": "John",
    "last_name": "Doe-Smith",
    "phone": "(555) 123-9999",
    "updated_at": "2024-01-16T11:00:00Z"
  }
}
```

#### Update User Role (Admin Only)
```http
PATCH /api/v1/profiles/{user_id}/role
Authorization: Bearer {admin_supabase_jwt_token}
Content-Type: application/json

{
  "role": "teacher",
  "reason": "Promoted to teacher position"
}

Response:
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "role": "teacher",
    "updated_at": "2024-01-16T11:30:00Z",
    "role_changed_by": "admin-uuid",
    "role_change_reason": "Promoted to teacher position"
  }
}
```

#### Deactivate User (Admin Only)
```http
PATCH /api/v1/profiles/{user_id}/status
Authorization: Bearer {admin_supabase_jwt_token}
Content-Type: application/json

{
  "is_active": false,
  "reason": "User requested account deactivation"
}

Response:
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "is_active": false,
    "deactivated_at": "2024-01-16T12:00:00Z",
    "deactivated_by": "admin-uuid",
    "deactivation_reason": "User requested account deactivation"
  }
}
```

#### Get User's Authentication Status
```http
GET /api/v1/profiles/{user_id}/auth-status
Authorization: Bearer {admin_supabase_jwt_token}

Response:
{
  "data": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "parent@example.com",
    "email_confirmed": true,
    "email_confirmed_at": "2024-01-01T00:30:00Z",
    "phone_confirmed": false,
    "last_sign_in_at": "2024-01-15T09:30:00Z",
    "sign_in_count": 47,
    "created_at": "2024-01-01T00:00:00Z",
    "auth_provider": "email",
    "mfa_enabled": false
  }
}
```

### 3.3 Children Management

#### List Children
```http
GET /api/v1/children?parent_id={parent_id}&limit=10
Authorization: Bearer {jwt_token}

Response:
{
  "data": [
    {
      "id": "child-uuid-1",
      "parent_id": "parent-uuid",
      "first_name": "Emma",
      "last_name": "Doe",
      "birth_date": "2020-05-15",
      "age_group": "toddler",
      "medical_info": {
        "allergies": ["peanuts", "dairy"],
        "medications": [],
        "special_needs": null,
        "emergency_medical_info": "Call 911 for severe allergic reaction"
      },
      "enrollment_status": "enrolled",
      "classroom": {
        "id": "classroom-uuid",
        "name": "Toddler Room A",
        "age_group": "toddler"
      },
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-10T14:30:00Z"
    }
  ]
}
```

#### Add Child
```http
POST /api/v1/children
Authorization: Bearer {parent_jwt_token}
Content-Type: application/json

{
  "first_name": "Oliver",
  "last_name": "Doe",
  "birth_date": "2021-03-10",
  "medical_info": {
    "allergies": [],
    "medications": ["Children's Tylenol as needed"],
    "special_needs": null,
    "pediatrician": {
      "name": "Dr. Smith",
      "phone": "(555) 111-2222"
    }
  }
}

Response:
{
  "data": {
    "id": "new-child-uuid",
    "parent_id": "parent-uuid",
    "first_name": "Oliver",
    "last_name": "Doe",
    "birth_date": "2021-03-10",
    "age_group": "toddler",
    "enrollment_status": "pending",
    "created_at": "2024-01-16T11:00:00Z"
  }
}
```

#### Update Child Information
```http
PATCH /api/v1/children/{child_id}
Authorization: Bearer {parent_jwt_token}
Content-Type: application/json

{
  "medical_info": {
    "allergies": ["peanuts"],
    "medications": ["Albuterol inhaler"],
    "emergency_medical_info": "Has asthma - inhaler in backpack"
  }
}
```

### 3.4 Classroom Management

#### List Classrooms
```http
GET /api/v1/classrooms
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "classroom-uuid-1",
      "name": "Toddler Room A",
      "age_group": "toddler",
      "capacity": 12,
      "enrolled_count": 8,
      "available_spots": 4,
      "teachers": [
        {
          "id": "teacher-uuid-1",
          "first_name": "Ms. Sarah",
          "last_name": "Johnson",
          "role": "lead_teacher"
        }
      ],
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### Create Classroom
```http
POST /api/v1/classrooms
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "name": "Preschool Room B",
  "age_group": "preschool",
  "capacity": 16,
  "description": "Ages 3-4 years old"
}
```

### 3.5 Enrollment Management

#### List Enrollments
```http
GET /api/v1/enrollments?status=pending&limit=20&sort=created_at
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "enrollment-uuid-1",
      "child": {
        "id": "child-uuid",
        "first_name": "Emma",
        "last_name": "Doe",
        "birth_date": "2020-05-15"
      },
      "parent": {
        "id": "parent-uuid",
        "first_name": "John",
        "last_name": "Doe",
        "email": "parent@example.com"
      },
      "classroom": {
        "id": "classroom-uuid",
        "name": "Toddler Room A",
        "age_group": "toddler"
      },
      "status": "pending",
      "progress": {
        "total_forms": 8,
        "completed_forms": 5,
        "pending_forms": 3,
        "completion_percentage": 62.5
      },
      "start_date": "2024-02-01",
      "created_at": "2024-01-16T10:00:00Z",
      "updated_at": "2024-01-16T15:30:00Z"
    }
  ]
}
```

#### Create Enrollment
```http
POST /api/v1/enrollments
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "child_id": "child-uuid",
  "classroom_id": "classroom-uuid",
  "start_date": "2024-02-01",
  "notes": "Child has experience with daycare"
}

Response:
{
  "data": {
    "id": "new-enrollment-uuid",
    "child_id": "child-uuid",
    "classroom_id": "classroom-uuid",
    "status": "pending",
    "start_date": "2024-02-01",
    "forms_assigned": 8,
    "created_at": "2024-01-16T12:00:00Z"
  }
}
```

#### Update Enrollment Status
```http
PATCH /api/v1/enrollments/{enrollment_id}
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "status": "approved",
  "admin_notes": "All forms completed and reviewed. Welcome to Brookside!",
  "notify_parent": true
}
```

#### Get Enrollment Details
```http
GET /api/v1/enrollments/{enrollment_id}
Authorization: Bearer {jwt_token}

Response:
{
  "data": {
    "id": "enrollment-uuid",
    "child": { /* child details */ },
    "parent": { /* parent details */ },
    "classroom": { /* classroom details */ },
    "status": "pending",
    "progress": {
      "total_forms": 8,
      "completed_forms": 6,
      "pending_forms": 2,
      "completion_percentage": 75.0,
      "last_activity": "2024-01-16T14:30:00Z"
    },
    "timeline": [
      {
        "event": "enrollment_created",
        "timestamp": "2024-01-16T10:00:00Z",
        "actor": "Admin User",
        "details": "Enrollment created for Toddler Room A"
      },
      {
        "event": "form_completed",
        "timestamp": "2024-01-16T14:30:00Z",
        "actor": "Parent",
        "details": "Completed Medical Information Form"
      }
    ],
    "start_date": "2024-02-01",
    "created_at": "2024-01-16T10:00:00Z",
    "updated_at": "2024-01-16T15:30:00Z"
  }
}
```

### 3.6 Admin Approval & Form Locking APIs

The admin approval system enables school administrators to review, approve, reject, or request revisions for enrollment applications. Once approved, all forms are automatically locked to prevent further parent edits, ensuring data integrity and compliance with enrollment policies.

#### Get Enrollments Pending Approval
```http
GET /api/v1/admin/enrollments/pending-approval?limit=20&sort=submitted_at
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "enrollment-uuid-1",
      "child": {
        "id": "child-uuid",
        "first_name": "Emma",
        "last_name": "Doe",
        "birth_date": "2020-05-15"
      },
      "parent": {
        "id": "parent-uuid",
        "first_name": "John",
        "last_name": "Doe",
        "email": "parent@example.com",
        "notification_emails": [
          "parent@example.com",
          "work@example.com",
          "backup@example.com"
        ]
      },
      "classroom": {
        "id": "classroom-uuid",
        "name": "Toddler Room A",
        "age_group": "toddler"
      },
      "admin_approval_status": "pending",
      "submitted_at": "2024-01-16T09:30:00Z",
      "forms_progress": {
        "total_required_forms": 8,
        "completed_required_forms": 8,
        "total_forms": 10,
        "completed_forms": 8,
        "completion_percentage": 100.0,
        "ready_for_approval": true
      },
      "start_date": "2024-02-01",
      "created_at": "2024-01-15T10:00:00Z",
      "days_pending": 1
    }
  ],
  "meta": {
    "total": 12,
    "ready_for_approval": 8,
    "average_processing_days": 2.5
  }
}
```

#### Approve Enrollment
```http
POST /api/v1/enrollments/{enrollment_id}/approve
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "approval_notes": "All forms completed successfully. Welcome to Brookside Goddard School!",
  "notify_parents": true,
  "lock_forms": true
}

Response:
{
  "data": {
    "enrollment_id": "enrollment-uuid",
    "admin_approval_status": "approved",
    "approved_at": "2024-01-16T14:30:00Z",
    "approved_by": {
      "id": "admin-uuid",
      "name": "Sarah Johnson",
      "role": "admin"
    },
    "approval_notes": "All forms completed successfully. Welcome to Brookside Goddard School!",
    "forms_locked": true,
    "forms_locked_at": "2024-01-16T14:30:00Z",
    "notifications_sent": {
      "parent_emails": [
        "parent@example.com",
        "work@example.com", 
        "backup@example.com"
      ],
      "notification_count": 3
    }
  }
}
```

#### Reject Enrollment
```http
POST /api/v1/enrollments/{enrollment_id}/reject
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "rejection_notes": "Medical immunization records are incomplete. Please provide updated vaccination documentation before resubmission.",
  "notify_parents": true,
  "unlock_forms": true
}

Response:
{
  "data": {
    "enrollment_id": "enrollment-uuid",
    "admin_approval_status": "rejected",
    "approved_by": {
      "id": "admin-uuid", 
      "name": "Sarah Johnson",
      "role": "admin"
    },
    "approval_notes": "Medical immunization records are incomplete. Please provide updated vaccination documentation before resubmission.",
    "forms_locked": false,
    "notifications_sent": {
      "parent_emails": [
        "parent@example.com",
        "work@example.com",
        "backup@example.com" 
      ],
      "notification_count": 3
    },
    "rejected_at": "2024-01-16T14:30:00Z"
  }
}
```

#### Request Enrollment Revision
```http
POST /api/v1/enrollments/{enrollment_id}/request-revision
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "revision_notes": "Please update the emergency contact information to include at least one local contact within 30 miles of the school.",
  "specific_forms": [
    "emergency-contact-form-uuid"
  ],
  "notify_parents": true
}

Response:
{
  "data": {
    "enrollment_id": "enrollment-uuid",
    "admin_approval_status": "needs_revision",
    "revision_requested_at": "2024-01-16T14:30:00Z",
    "requested_by": {
      "id": "admin-uuid",
      "name": "Sarah Johnson", 
      "role": "admin"
    },
    "revision_notes": "Please update the emergency contact information to include at least one local contact within 30 miles of the school.",
    "forms_unlocked": [
      "emergency-contact-form-uuid"
    ],
    "forms_locked": false,
    "notifications_sent": {
      "parent_emails": [
        "parent@example.com",
        "work@example.com",
        "backup@example.com"
      ],
      "notification_count": 3
    }
  }
}
```

#### Get Enrollment Approval Status
```http
GET /api/v1/enrollments/{enrollment_id}/approval-status
Authorization: Bearer {jwt_token}

Response:
{
  "data": {
    "enrollment_id": "enrollment-uuid",
    "admin_approval_status": "approved",
    "approved_at": "2024-01-16T14:30:00Z",
    "approved_by": {
      "name": "Sarah Johnson",
      "role": "admin"
    },
    "approval_notes": "All forms completed successfully. Welcome to Brookside Goddard School!",
    "forms_locked": true,
    "forms_locked_at": "2024-01-16T14:30:00Z",
    "revision_notes": null,
    "forms_progress": {
      "required_forms_count": 8,
      "completed_forms_count": 8,
      "can_be_approved": true
    }
  }
}
```

#### Get Enrollment Approval History
```http
GET /api/v1/enrollments/{enrollment_id}/approval-history
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "audit-uuid-3",
      "action": "approve",
      "previous_status": "pending",
      "new_status": "approved",
      "admin": {
        "id": "admin-uuid",
        "name": "Sarah Johnson",
        "role": "admin"
      },
      "notes": "All forms completed successfully. Welcome to Brookside Goddard School!",
      "affected_forms": [],
      "created_at": "2024-01-16T14:30:00Z"
    },
    {
      "id": "audit-uuid-2", 
      "action": "request_revision",
      "previous_status": "pending",
      "new_status": "needs_revision",
      "admin": {
        "id": "admin-uuid-2",
        "name": "Michael Chen",
        "role": "admin"
      },
      "notes": "Please update emergency contact information.",
      "affected_forms": [
        "emergency-contact-form-uuid"
      ],
      "created_at": "2024-01-15T16:45:00Z"
    },
    {
      "id": "audit-uuid-1",
      "action": "submit_for_review",
      "previous_status": "draft",
      "new_status": "pending",
      "admin": null,
      "notes": "Parent completed all required forms",
      "affected_forms": [],
      "created_at": "2024-01-15T10:00:00Z"
    }
  ]
}
```

#### Check Form Lock Status
```http
GET /api/v1/enrollments/{enrollment_id}/forms/lock-status
Authorization: Bearer {jwt_token}

Response:
{
  "data": {
    "enrollment_id": "enrollment-uuid",
    "forms_locked": true,
    "forms_locked_at": "2024-01-16T14:30:00Z",
    "approval_status": "approved",
    "can_edit_forms": false,
    "lock_reason": "Enrollment has been approved by school administration",
    "locked_forms": [
      {
        "assignment_id": "assignment-uuid-1",
        "form_name": "Admission Application",
        "locked_at": "2024-01-16T14:30:00Z"
      },
      {
        "assignment_id": "assignment-uuid-2", 
        "form_name": "Medical Information",
        "locked_at": "2024-01-16T14:30:00Z"
      }
    ]
  }
}
```

#### Get Approval Statistics
```http
GET /api/v1/admin/approval-statistics?period=30days
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "period": "30days",
    "statistics": {
      "total_enrollments": 45,
      "pending_approval": 8,
      "approved": 32,
      "rejected": 3,
      "needs_revision": 2,
      "approval_rates": {
        "approved_percentage": 71.1,
        "rejected_percentage": 6.7,
        "revision_percentage": 4.4
      },
      "average_approval_time_days": 2.3,
      "fastest_approval_hours": 4.5,
      "slowest_approval_days": 7.2
    },
    "trends": [
      {
        "date": "2024-01-01",
        "approved": 3,
        "rejected": 0,
        "needs_revision": 1
      },
      {
        "date": "2024-01-02", 
        "approved": 5,
        "rejected": 1,
        "needs_revision": 0
      }
    ]
  }
}
```

---

## 4. Form Management APIs

The form management system implements a sophisticated 3-tier hierarchy with integrated approval workflow and form locking:
1. **Form Templates**: School-level form definitions
2. **Class Form Overrides**: Classroom-specific include/exclude rules  
3. **Student Form Assignments**: Materialized individual assignments with approval status

### 4.1 Form Templates

Form templates define the available forms at the school level with different states.

#### List Form Templates
```http
GET /api/v1/form-templates?status=active&type=admission
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "form-template-uuid-1",
      "form_name": "Admission Application",
      "form_description": "Complete application for school admission",
      "form_type": "admission",
      "fillout_form_id": "fillout-form-123",
      "fillout_form_url": "https://forms.brookside.goddard.com/admission",
      "status": "school_default",
      "is_required": true,
      "display_order": 1,
      "assignments_count": 45,
      "completion_rate": 78.5,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-10T09:15:00Z"
    }
  ]
}
```

#### Create Form Template
```http
POST /api/v1/form-templates
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "form_name": "Emergency Contact Information",
  "form_description": "Emergency contact details and authorization",
  "form_type": "emergency",
  "fillout_form_id": "fillout-emergency-456",
  "fillout_form_url": "https://forms.brookside.goddard.com/emergency-contacts",
  "status": "active",
  "is_required": true,
  "display_order": 3
}

Response:
{
  "data": {
    "id": "new-form-template-uuid",
    "form_name": "Emergency Contact Information",
    "form_type": "emergency",
    "fillout_form_id": "fillout-emergency-456",
    "status": "active",
    "is_required": true,
    "created_at": "2024-01-16T13:00:00Z"
  }
}
```

#### Update Form Template
```http
PATCH /api/v1/form-templates/{template_id}
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "status": "school_default",
  "is_required": true,
  "display_order": 2
}
```

#### Get Form Template Stats
```http
GET /api/v1/form-templates/{template_id}/stats
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "template_id": "form-template-uuid",
    "total_assignments": 50,
    "completed_assignments": 42,
    "pending_assignments": 8,
    "completion_rate": 84.0,
    "average_completion_time_hours": 24.5,
    "assignments_by_source": {
      "school_default": 45,
      "class_override": 3,
      "individual": 2
    },
    "completion_trend": [
      {"date": "2024-01-10", "completed": 5},
      {"date": "2024-01-11", "completed": 8},
      {"date": "2024-01-12", "completed": 12}
    ]
  }
}
```

### 4.2 Class Form Overrides

Classroom-level form assignment rules that modify school defaults.

#### List Class Form Overrides
```http
GET /api/v1/classrooms/{classroom_id}/form-overrides
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "override-uuid-1",
      "classroom": {
        "id": "classroom-uuid",
        "name": "Toddler Room A",
        "age_group": "toddler"
      },
      "form_template": {
        "id": "form-template-uuid",
        "form_name": "Field Trip Permission",
        "form_type": "authorization"
      },
      "action": "exclude",
      "is_required": false,
      "reason": "Toddlers don't go on field trips",
      "created_at": "2024-01-05T10:00:00Z"
    }
  ]
}
```

#### Create Class Form Override
```http
POST /api/v1/classrooms/{classroom_id}/form-overrides
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "form_template_id": "form-template-uuid",
  "action": "include",
  "is_required": true,
  "reason": "Preschool age group requires additional allergy information"
}

Response:
{
  "data": {
    "id": "new-override-uuid",
    "classroom_id": "classroom-uuid",
    "form_template_id": "form-template-uuid",
    "action": "include",
    "is_required": true,
    "affected_enrollments": 12,
    "created_at": "2024-01-16T14:00:00Z"
  }
}
```

#### Update Class Form Override
```http
PATCH /api/v1/form-overrides/{override_id}
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "action": "exclude",
  "is_required": false,
  "reason": "Updated policy - form no longer needed for this age group"
}
```

#### Delete Class Form Override
```http
DELETE /api/v1/form-overrides/{override_id}
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "message": "Form override deleted successfully",
    "affected_enrollments": 8,
    "revert_to_default": true
  }
}
```

### 4.3 Student Form Assignments

Materialized assignments representing the final form list for each student.

#### List Student Form Assignments
```http
GET /api/v1/enrollments/{enrollment_id}/form-assignments?status=pending
Authorization: Bearer {jwt_token}

Response:
{
  "data": [
    {
      "id": "assignment-uuid-1",
      "form_template": {
        "id": "form-template-uuid",
        "form_name": "Admission Application",
        "form_type": "admission",
        "form_description": "Complete application for school admission",
        "fillout_form_url": "https://forms.brookside.goddard.com/admission?enrollment_id=enrollment-uuid&assignment_id=assignment-uuid-1"
      },
      "assignment_source": "school_default",
      "is_required": true,
      "status": "pending",
      "can_edit": true,
      "lock_reason": null,
      "assigned_at": "2024-01-16T10:00:00Z",
      "started_at": null,
      "completed_at": null,
      "form_submission": null
    },
    {
      "id": "assignment-uuid-2",
      "form_template": {
        "id": "form-template-uuid-2",
        "form_name": "Medical Information",
        "form_type": "medical",
        "fillout_form_url": "https://forms.brookside.goddard.com/medical?enrollment_id=enrollment-uuid&assignment_id=assignment-uuid-2"
      },
      "assignment_source": "school_default", 
      "is_required": true,
      "status": "completed",
      "can_edit": false,
      "lock_reason": "Form completed and enrollment approved",
      "assigned_at": "2024-01-16T10:00:00Z",
      "started_at": "2024-01-16T12:30:00Z", 
      "completed_at": "2024-01-16T14:15:00Z",
      "form_submission": {
        "id": "submission-uuid",
        "fillout_submission_id": "fillout-submission-789",
        "submitted_at": "2024-01-16T14:15:00Z"
      }
    }
  ]
}
```

#### Add Individual Form Assignment
```http
POST /api/v1/enrollments/{enrollment_id}/form-assignments
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "form_template_id": "form-template-uuid",
  "is_required": false,
  "reason": "Parent requested additional allergy documentation"
}

Response:
{
  "data": {
    "id": "new-assignment-uuid",
    "form_template_id": "form-template-uuid",
    "assignment_source": "individual",
    "is_required": false,
    "status": "pending",
    "assigned_at": "2024-01-16T15:00:00Z"
  }
}
```

#### Get Form Assignment Details
```http
GET /api/v1/form-assignments/{assignment_id}
Authorization: Bearer {jwt_token}

Response:
{
  "data": {
    "id": "assignment-uuid",
    "enrollment": {
      "id": "enrollment-uuid",
      "child": { /* child details */ },
      "parent": { /* parent details */ },
      "admin_approval_status": "approved",
      "forms_locked": true,
      "forms_locked_at": "2024-01-16T14:30:00Z"
    },
    "form_template": { /* form template details */ },
    "assignment_source": "school_default",
    "is_required": true,
    "status": "completed",
    "can_edit": false,
    "lock_reason": "Enrollment approved - forms locked by admin",
    "assigned_at": "2024-01-16T10:00:00Z",
    "started_at": "2024-01-16T12:30:00Z",
    "completed_at": "2024-01-16T14:15:00Z",
    "form_submission": {
      "id": "submission-uuid",
      "fillout_submission_id": "fillout-submission-789",
      "form_data": {
        "child_name": "Emma Doe",
        "birth_date": "2020-05-15",
        "allergies": ["peanuts", "dairy"]
      },
      "metadata": {
        "ip_address": "192.168.1.1",
        "user_agent": "Mozilla/5.0...",
        "completion_time_minutes": 23
      },
      "submitted_at": "2024-01-16T14:15:00Z",
      "processed_at": "2024-01-16T14:15:30Z"
    }
  }
}
```

### 4.4 Form Submissions (Fillout Webhooks)

Form submissions are processed via Fillout webhooks and linked to assignments.

#### Process Fillout Webhook
```http
POST /api/v1/webhooks/fillout
Content-Type: application/json
X-Fillout-Signature: sha256=webhook_signature

{
  "form_id": "fillout-form-123",
  "submission_id": "fillout-submission-789",
  "created_at": "2024-01-16T14:15:00Z",
  "data": {
    "enrollment_id": "enrollment-uuid",
    "assignment_id": "assignment-uuid",
    "child_name": "Emma Doe",
    "birth_date": "2020-05-15",
    "parent_name": "John Doe",
    "allergies": ["peanuts", "dairy"],
    "emergency_contact": {
      "name": "Jane Doe",
      "phone": "(555) 987-6543",
      "relationship": "mother"
    }
  },
  "metadata": {
    "ip_address": "192.168.1.1",
    "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)..."
  }
}

Response:
{
  "status": "success",
  "submission_processed": true,
  "assignment_completed": true,
  "forms_locked": false,
  "enrollment_approval_status": "pending"
}

Response (when forms are locked):
{
  "status": "error",
  "error": {
    "type": "https://api.goddard.com/errors/form-locked",
    "title": "Form Submission Not Allowed", 
    "status": 403,
    "detail": "Cannot process form submission - enrollment has been approved and forms are locked",
    "enrollment_id": "enrollment-uuid",
    "approval_status": "approved",
    "forms_locked_at": "2024-01-16T14:30:00Z"
  }
}
```

#### Form Locking Validation Rules

**Critical Validation:** All form submission endpoints must validate that forms are not locked before accepting submissions.

**Validation Logic:**
1. Check enrollment approval status
2. If `admin_approval_status = 'approved'` AND `forms_locked_at IS NOT NULL`, reject submission  
3. Return appropriate error response with form lock details

**Implementation Pattern (Rust):**
```rust
#[derive(Error, Debug)]
pub enum FormSubmissionError {
    #[error("Forms are locked - enrollment approved on {approved_at}")]
    FormsLocked { 
        enrollment_id: Uuid,
        approved_at: DateTime<Utc>,
        approved_by: String 
    },
    #[error("Enrollment not found or not accessible")]
    EnrollmentNotFound,
    #[error("Form assignment not found or completed")]
    AssignmentNotFound,
}

pub async fn validate_form_submission_allowed(
    enrollment_id: Uuid,
    assignment_id: Uuid,
    db: &Database
) -> Result<(), FormSubmissionError> {
    let enrollment = db.get_enrollment_approval_status(enrollment_id)
        .await?
        .ok_or(FormSubmissionError::EnrollmentNotFound)?;
    
    // Check if forms are locked
    if enrollment.admin_approval_status == "approved" && enrollment.forms_locked_at.is_some() {
        return Err(FormSubmissionError::FormsLocked {
            enrollment_id,
            approved_at: enrollment.approved_at.unwrap(),
            approved_by: enrollment.approved_by_name.unwrap_or("Admin".to_string()),
        });
    }
    
    Ok(())
}
```

#### List Form Submissions
```http
GET /api/v1/form-submissions?enrollment_id={enrollment_id}&limit=10
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "submission-uuid",
      "enrollment": { /* enrollment details */ },
      "form_template": { /* form template details */ },
      "fillout_submission_id": "fillout-submission-789",
      "form_data": { /* submitted form data */ },
      "metadata": { /* submission metadata */ },
      "submitted_at": "2024-01-16T14:15:00Z",
      "processed_at": "2024-01-16T14:15:30Z"
    }
  ]
}
```

#### Get Form Submission Details
```http
GET /api/v1/form-submissions/{submission_id}
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "id": "submission-uuid",
    "enrollment": { /* full enrollment details */ },
    "student_form_assignment": { /* assignment details */ },
    "form_template": { /* form template details */ },
    "fillout_submission_id": "fillout-submission-789",
    "form_data": { /* complete submitted data */ },
    "metadata": {
      "ip_address": "192.168.1.1",
      "user_agent": "Mozilla/5.0...",
      "referrer": "https://brookside.goddard.com/enroll",
      "completion_time_minutes": 23,
      "device_type": "desktop"
    },
    "submitted_at": "2024-01-16T14:15:00Z",
    "processed_at": "2024-01-16T14:15:30Z"
  }
}
```

---

## 5. Communication & Document APIs

### 5.1 Parent Additional Emails

The parent additional email system allows administrators to manage multiple email addresses per parent for enhanced communication reach.

#### List Parent Additional Emails
```http
GET /api/v1/parents/{parent_id}/additional-emails
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "email-uuid-1",
      "parent_id": "parent-uuid",
      "email_address": "work@example.com",
      "email_type": "work",
      "is_verified": true,
      "is_active": true,
      "added_by": {
        "id": "admin-uuid",
        "first_name": "Admin",
        "last_name": "User",
        "email": "admin@brookside.goddard.com"
      },
      "notes": "Work email for urgent communications",
      "created_at": "2024-01-10T09:00:00Z",
      "updated_at": "2024-01-12T14:30:00Z"
    }
  ]
}
```

#### Add Additional Email for Parent
```http
POST /api/v1/parents/{parent_id}/additional-emails
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "email_address": "emergency@example.com",
  "email_type": "emergency",
  "notes": "Emergency contact - grandmother's email"
}

Response:
{
  "data": {
    "id": "new-email-uuid",
    "parent_id": "parent-uuid",
    "email_address": "emergency@example.com",
    "email_type": "emergency",
    "is_verified": false,
    "is_active": true,
    "added_by": {
      "id": "admin-uuid",
      "first_name": "Admin",
      "last_name": "User"
    },
    "notes": "Emergency contact - grandmother's email",
    "created_at": "2024-01-16T16:00:00Z"
  }
}
```

#### Update Additional Email Status
```http
PATCH /api/v1/additional-emails/{email_id}
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "is_active": false,
  "notes": "Email no longer active per parent request"
}
```

#### Get All Notification Emails for Parent
```http
GET /api/v1/parents/{parent_id}/notification-emails
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "parent_id": "parent-uuid",
    "primary_email": "parent@example.com",
    "additional_emails": [
      {
        "email_address": "work@example.com",
        "email_type": "work",
        "is_active": true,
        "is_verified": true
      },
      {
        "email_address": "emergency@example.com",
        "email_type": "emergency",
        "is_active": true,
        "is_verified": false
      }
    ],
    "total_active_emails": 3,
    "notification_ready_emails": [
      "parent@example.com",
      "work@example.com"
    ]
  }
}
```

### 5.2 Email Notifications

#### Send Custom Email
```http
POST /api/v1/notifications/email
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "recipients": [
    {
      "parent_id": "parent-uuid-1",
      "email_types": ["primary", "work"]
    },
    {
      "parent_id": "parent-uuid-2",
      "email_types": ["primary"]
    }
  ],
  "template_type": "custom",
  "subject": "Important Update About School Policies",
  "message": "Dear parents, we wanted to inform you about updated pickup procedures...",
  "include_school_branding": true,
  "send_immediately": true
}

Response:
{
  "data": {
    "notification_id": "notification-uuid",
    "recipients_count": 3,
    "emails_sent": 3,
    "emails_failed": 0,
    "status": "sent",
    "sent_at": "2024-01-16T17:00:00Z",
    "delivery_tracking": [
      {
        "email": "parent1@example.com",
        "status": "delivered",
        "delivered_at": "2024-01-16T17:01:00Z"
      },
      {
        "email": "work1@example.com",
        "status": "delivered",
        "delivered_at": "2024-01-16T17:01:15Z"
      },
      {
        "email": "parent2@example.com",
        "status": "delivered",
        "delivered_at": "2024-01-16T17:01:30Z"
      }
    ]
  }
}
```

#### Send Enrollment Reminder
```http
POST /api/v1/notifications/enrollment-reminder
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "enrollment_ids": ["enrollment-uuid-1", "enrollment-uuid-2"],
  "reminder_type": "forms_pending",
  "custom_message": "Please complete the remaining forms to finish your enrollment.",
  "include_form_links": true
}
```

#### Get Notification History
```http
GET /api/v1/notifications?parent_id={parent_id}&limit=20
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "notification-uuid",
      "type": "enrollment_invitation",
      "subject": "Welcome to Brookside Goddard School!",
      "recipient_emails": ["parent@example.com", "work@example.com"],
      "status": "delivered",
      "sent_at": "2024-01-16T10:00:00Z",
      "opened_at": "2024-01-16T10:15:00Z",
      "clicked_at": "2024-01-16T10:16:00Z"
    }
  ]
}
```

### 5.3 Document Management

#### Upload Document
```http
POST /api/v1/enrollments/{enrollment_id}/documents
Authorization: Bearer {parent_jwt_token}
Content-Type: multipart/form-data

{
  "file": [binary file data],
  "document_type": "immunization_record",
  "description": "Updated immunization record for Emma Doe",
  "file_name": "emma_immunizations_2024.pdf"
}

Response:
{
  "data": {
    "id": "document-uuid",
    "enrollment_id": "enrollment-uuid",
    "document_type": "immunization_record",
    "file_name": "emma_immunizations_2024.pdf",
    "original_file_name": "emma_immunizations_2024.pdf",
    "file_size": 245760,
    "mime_type": "application/pdf",
    "storage_path": "schools/brookside/enrollments/enrollment-uuid/documents/document-uuid.pdf",
    "description": "Updated immunization record for Emma Doe",
    "uploaded_by": {
      "id": "parent-uuid",
      "first_name": "John",
      "last_name": "Doe",
      "role": "parent"
    },
    "is_verified": false,
    "uploaded_at": "2024-01-16T18:00:00Z"
  }
}
```

#### List Documents
```http
GET /api/v1/enrollments/{enrollment_id}/documents?type=immunization_record
Authorization: Bearer {jwt_token}

Response:
{
  "data": [
    {
      "id": "document-uuid",
      "document_type": "immunization_record",
      "file_name": "emma_immunizations_2024.pdf",
      "file_size": 245760,
      "mime_type": "application/pdf",
      "description": "Updated immunization record for Emma Doe",
      "uploaded_by": { /* user details */ },
      "is_verified": false,
      "verified_by": null,
      "uploaded_at": "2024-01-16T18:00:00Z",
      "download_url": "https://api.goddard.com/v1/documents/document-uuid/download"
    }
  ]
}
```

#### Download Document
```http
GET /api/v1/documents/{document_id}/download
Authorization: Bearer {jwt_token}

# Returns binary file content with appropriate headers
Content-Type: application/pdf
Content-Disposition: attachment; filename="emma_immunizations_2024.pdf"
```

#### Verify Document (Admin Only)
```http
PATCH /api/v1/documents/{document_id}/verify
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "is_verified": true,
  "admin_notes": "Immunization record reviewed and approved. All required vaccines current."
}
```

#### Delete Document
```http
DELETE /api/v1/documents/{document_id}
Authorization: Bearer {jwt_token}

Response:
{
  "data": {
    "message": "Document deleted successfully",
    "document_id": "document-uuid",
    "deleted_at": "2024-01-16T19:00:00Z"
  }
}
```

### 5.4 Communication Templates

#### List Email Templates
```http
GET /api/v1/communication/templates?type=email
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "template-uuid-1",
      "name": "enrollment_invitation",
      "display_name": "Enrollment Invitation",
      "type": "email",
      "subject": "Welcome to {{school_name}}!",
      "content": "Dear {{parent_name}}, we're excited to welcome {{child_name}} to {{school_name}}...",
      "variables": [
        {"name": "school_name", "description": "Name of the school"},
        {"name": "parent_name", "description": "Parent's first name"},
        {"name": "child_name", "description": "Child's first name"},
        {"name": "enrollment_url", "description": "Link to enrollment portal"}
      ],
      "is_system": true,
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-10T14:00:00Z"
    }
  ]
}
```

#### Create Custom Template
```http
POST /api/v1/communication/templates
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "name": "late_pickup_reminder",
  "display_name": "Late Pickup Reminder",
  "type": "email",
  "subject": "Pickup Reminder for {{child_name}}",
  "content": "Dear {{parent_name}}, this is a friendly reminder that pickup time is at {{pickup_time}}. Please plan accordingly.",
  "variables": [
    {"name": "parent_name", "description": "Parent's name"},
    {"name": "child_name", "description": "Child's name"},
    {"name": "pickup_time", "description": "Scheduled pickup time"}
  ]
}
```

---

## 6. Administrative APIs

### 6.1 Dashboard Analytics

#### Get School Dashboard Overview
```http
GET /api/v1/admin/dashboard/overview
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "school": {
      "id": "school-uuid",
      "name": "Brookside Goddard School",
      "total_capacity": 120,
      "current_enrollment": 95,
      "available_spots": 25,
      "occupancy_rate": 79.2
    },
    "enrollment_stats": {
      "total_enrollments": 105,
      "pending_enrollments": 8,
      "approved_enrollments": 95,
      "rejected_enrollments": 2,
      "completion_rate": 85.7,
      "average_completion_time_days": 3.2
    },
    "form_stats": {
      "total_forms_assigned": 840,
      "completed_forms": 720,
      "pending_forms": 120,
      "completion_rate": 85.7,
      "most_completed_form": "Admission Application",
      "least_completed_form": "Photo Release Authorization"
    },
    "recent_activity": [
      {
        "type": "enrollment_completed",
        "message": "Emma Doe completed all enrollment forms",
        "timestamp": "2024-01-16T16:30:00Z",
        "enrollment_id": "enrollment-uuid"
      },
      {
        "type": "document_uploaded",
        "message": "John Smith uploaded immunization record",
        "timestamp": "2024-01-16T15:45:00Z",
        "document_id": "document-uuid"
      }
    ],
    "classroom_stats": [
      {
        "classroom_id": "classroom-uuid-1",
        "name": "Toddler Room A",
        "capacity": 12,
        "enrolled": 10,
        "pending": 2,
        "occupancy_rate": 83.3
      }
    ],
    "generated_at": "2024-01-16T20:00:00Z"
  }
}
```

#### Get Enrollment Metrics
```http
GET /api/v1/admin/analytics/enrollments?period=30d&group_by=day
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "period": "30d",
    "group_by": "day",
    "metrics": {
      "enrollments_created": [
        {"date": "2024-01-01", "count": 3},
        {"date": "2024-01-02", "count": 5},
        {"date": "2024-01-03", "count": 2}
      ],
      "enrollments_completed": [
        {"date": "2024-01-01", "count": 2},
        {"date": "2024-01-02", "count": 4},
        {"date": "2024-01-03", "count": 3}
      ],
      "form_completion_rate": [
        {"date": "2024-01-01", "rate": 85.5},
        {"date": "2024-01-02", "rate": 87.2},
        {"date": "2024-01-03", "rate": 89.1}
      ]
    },
    "summary": {
      "total_enrollments": 45,
      "completed_enrollments": 38,
      "average_completion_rate": 87.3,
      "trend": "improving"
    }
  }
}
```

#### Get Form Completion Analytics
```http
GET /api/v1/admin/analytics/forms?form_template_id={template_id}
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "form_template": {
      "id": "form-template-uuid",
      "form_name": "Admission Application",
      "form_type": "admission"
    },
    "completion_stats": {
      "total_assignments": 50,
      "completed_assignments": 45,
      "pending_assignments": 5,
      "completion_rate": 90.0,
      "average_completion_time_hours": 12.5
    },
    "completion_funnel": [
      {"step": "form_assigned", "count": 50, "percentage": 100.0},
      {"step": "form_started", "count": 48, "percentage": 96.0},
      {"step": "form_completed", "count": 45, "percentage": 90.0}
    ],
    "drop_off_points": [
      {"section": "Personal Information", "drop_off_rate": 2.0},
      {"section": "Medical Information", "drop_off_rate": 4.0},
      {"section": "Emergency Contacts", "drop_off_rate": 4.0}
    ],
    "completion_time_distribution": {
      "under_1_hour": 12,
      "1_to_6_hours": 18,
      "6_to_24_hours": 10,
      "1_to_3_days": 4,
      "over_3_days": 1
    }
  }
}
```

### 6.2 Reporting APIs

#### Generate Enrollment Report
```http
POST /api/v1/admin/reports/enrollments
Authorization: Bearer {admin_jwt_token}
Content-Type: application/json

{
  "format": "xlsx",
  "date_range": {
    "start_date": "2024-01-01",
    "end_date": "2024-01-31"
  },
  "filters": {
    "status": ["pending", "approved"],
    "classroom_ids": ["classroom-uuid-1", "classroom-uuid-2"]
  },
  "include_fields": [
    "child_info",
    "parent_info",
    "form_completion_status",
    "documents_status",
    "enrollment_timeline"
  ],
  "email_delivery": {
    "recipient": "admin@brookside.goddard.com",
    "subject": "Monthly Enrollment Report - January 2024"
  }
}

Response:
{
  "data": {
    "report_id": "report-uuid",
    "status": "generating",
    "format": "xlsx",
    "estimated_completion": "2024-01-16T20:05:00Z",
    "download_url": null,
    "email_delivery": true,
    "created_at": "2024-01-16T20:00:00Z"
  }
}
```

#### Get Report Status
```http
GET /api/v1/admin/reports/{report_id}
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "id": "report-uuid",
    "type": "enrollments",
    "status": "completed",
    "format": "xlsx",
    "file_size": 125440,
    "record_count": 45,
    "download_url": "https://api.goddard.com/v1/reports/report-uuid/download",
    "expires_at": "2024-01-23T20:00:00Z",
    "created_at": "2024-01-16T20:00:00Z",
    "completed_at": "2024-01-16T20:03:00Z"
  }
}
```

#### Export Parent Contact List
```http
GET /api/v1/admin/exports/parent-contacts?format=csv
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": {
    "export_id": "export-uuid",
    "type": "parent_contacts",
    "status": "completed",
    "download_url": "https://api.goddard.com/v1/exports/export-uuid/download",
    "record_count": 75,
    "includes_additional_emails": true,
    "generated_at": "2024-01-16T20:10:00Z"
  }
}
```

### 6.3 Audit & Compliance

#### Get Audit Log
```http
GET /api/v1/admin/audit-log?user_id={user_id}&action=form_completed&limit=50
Authorization: Bearer {admin_jwt_token}

Response:
{
  "data": [
    {
      "id": "audit-uuid-1",
      "timestamp": "2024-01-16T14:15:00Z",
      "user": {
        "id": "parent-uuid",
        "email": "parent@example.com",
        "role": "parent",
        "name": "John Doe"
      },
      "action": "form_completed",
      "resource_type": "form_submission",
      "resource_id": "submission-uuid",
      "details": {
        "form_name": "Medical Information",
        "enrollment_id": "enrollment-uuid",
        "child_name": "Emma Doe",
        "completion_time_minutes": 23
      },
      "ip_address": "192.168.1.1",
      "user_agent": "Mozilla/5.0...",
      "session_id": "session-uuid"
    }
  ]
}
```

#### Data Retention Report
```http
GET /api/v1/admin/compliance/data-retention
Authorization: Bearer {super_admin_jwt_token}

Response:
{
  "data": {
    "school_id": "school-uuid",
    "retention_policy": {
      "enrollment_records": "7_years",
      "form_submissions": "7_years",
      "audit_logs": "3_years",
      "documents": "7_years"
    },
    "records_summary": {
      "total_records": 15420,
      "records_due_for_deletion": 23,
      "oldest_record_date": "2017-08-15",
      "retention_compliance": "compliant"
    },
    "upcoming_deletions": [
      {
        "record_type": "enrollment",
        "record_id": "old-enrollment-uuid",
        "deletion_date": "2024-02-01",
        "reason": "7_year_retention_limit"
      }
    ]
  }
}
```

---

## 7. Error Handling

### 7.1 Standard Error Response Format (RFC 7807)

All error responses follow the RFC 7807 Problem Details for HTTP APIs standard:

```json
{
  "type": "https://api.goddard.com/errors/validation-error",
  "title": "Validation Error",
  "status": 400,
  "detail": "One or more fields failed validation",
  "instance": "/api/v1/children",
  "school_id": "550e8400-e29b-41d4-a716-446655440000",
  "request_id": "req-12345-67890",
  "timestamp": "2024-01-16T20:30:00Z",
  "errors": [
    {
      "field": "birth_date",
      "code": "invalid_date",
      "message": "Birth date must be in the past"
    },
    {
      "field": "first_name",
      "code": "required",
      "message": "First name is required"
    }
  ]
}
```

### 7.2 Common Error Types

#### Authentication Errors (401)
```json
{
  "type": "https://api.goddard.com/errors/authentication-error",
  "title": "Authentication Required",
  "status": 401,
  "detail": "Valid authentication token is required to access this resource",
  "instance": "/api/v1/children"
}
```

#### Authorization Errors (403)
```json
{
  "type": "https://api.goddard.com/errors/authorization-error",
  "title": "Insufficient Permissions",
  "status": 403,
  "detail": "You do not have permission to access children from other schools",
  "instance": "/api/v1/children/other-school-child-id",
  "required_permission": "admin_role",
  "current_permissions": ["parent_role"]
}
```

#### Resource Not Found (404)
```json
{
  "type": "https://api.goddard.com/errors/not-found",
  "title": "Resource Not Found",
  "status": 404,
  "detail": "The requested enrollment could not be found",
  "instance": "/api/v1/enrollments/non-existent-uuid",
  "resource_type": "enrollment",
  "resource_id": "non-existent-uuid"
}
```

#### Validation Errors (422)
```json
{
  "type": "https://api.goddard.com/errors/validation-error",
  "title": "Validation Error",
  "status": 422,
  "detail": "The request data failed validation rules",
  "instance": "/api/v1/form-templates",
  "errors": [
    {
      "field": "fillout_form_id",
      "code": "duplicate",
      "message": "This Fillout form ID is already in use by another template"
    },
    {
      "field": "status",
      "code": "invalid_transition",
      "message": "Cannot change status from 'archive' directly to 'school_default'"
    }
  ]
}
```

#### Rate Limit Errors (429)
```json
{
  "type": "https://api.goddard.com/errors/rate-limit",
  "title": "Rate Limit Exceeded",
  "status": 429,
  "detail": "Request rate limit exceeded for this school",
  "instance": "/api/v1/notifications/email",
  "retry_after": 60,
  "limit": 1000,
  "remaining": 0,
  "reset_time": "2024-01-16T21:00:00Z"
}
```

#### Multi-Tenancy Violations (403)
```json
{
  "type": "https://api.goddard.com/errors/tenant-isolation",
  "title": "Cross-Tenant Access Denied",
  "status": 403,
  "detail": "Cannot access resources from other schools",
  "instance": "/api/v1/enrollments/other-school-enrollment-id",
  "user_school_id": "current-school-uuid",
  "resource_school_id": "other-school-uuid"
}
```

### 7.3 Error Handling Best Practices

#### Client Error Handling Pattern
```typescript
interface ApiError {
  type: string;
  title: string;
  status: number;
  detail: string;
  instance: string;
  school_id?: string;
  request_id?: string;
  timestamp?: string;
  errors?: Array<{
    field: string;
    code: string;
    message: string;
  }>;
}

// Example error handling in TypeScript
async function handleApiRequest<T>(request: Promise<T>): Promise<T> {
  try {
    return await request;
  } catch (error) {
    const apiError = error.response?.data as ApiError;
    
    switch (apiError.status) {
      case 401:
        // Redirect to login
        redirectToLogin();
        break;
      case 403:
        // Show permission denied message
        showPermissionError(apiError);
        break;
      case 422:
        // Handle validation errors
        handleValidationErrors(apiError.errors || []);
        break;
      case 429:
        // Wait and retry
        await wait(apiError.retry_after * 1000);
        return handleApiRequest(request);
      default:
        // Generic error handling
        showGenericError(apiError);
    }
    
    throw error;
  }
}
```

---

## 8. Implementation Guide

### 8.1 Rust Implementation Structure

#### Project Structure
```
src/
├── main.rs                 # Lambda entry point
├── lib.rs                  # Library exports
├── handlers/               # Lambda handlers
│   ├── mod.rs
│   ├── profiles.rs        # User profile management
│   ├── children.rs        # Children management
│   ├── enrollments.rs     # Enrollment management
│   ├── forms.rs           # Form management
│   ├── documents.rs       # Document management
│   ├── notifications.rs   # Email notifications
│   ├── invites.rs         # User invitations
│   └── webhooks.rs        # Fillout webhooks
├── models/                # Data models
│   ├── mod.rs
│   ├── profile.rs         # User profiles
│   ├── child.rs
│   ├── enrollment.rs
│   ├── form.rs
│   └── document.rs
├── services/              # Business logic
│   ├── mod.rs
│   ├── profile_service.rs # Profile management
│   ├── enrollment_service.rs
│   ├── form_service.rs
│   ├── invite_service.rs  # User invitation logic
│   └── notification_service.rs
├── database/              # Database layer
│   ├── mod.rs
│   ├── connection.rs
│   ├── migrations.rs
│   └── repositories/
│       ├── mod.rs
│       ├── profile_repository.rs
│       ├── child_repository.rs
│       └── enrollment_repository.rs
├── middleware/            # HTTP middleware
│   ├── mod.rs
│   ├── supabase_auth.rs  # Supabase JWT validation
│   ├── tenant.rs         # Multi-tenant context
│   └── rate_limit.rs     # Rate limiting
├── utils/                 # Utilities
│   ├── mod.rs
│   ├── error.rs          # Error handling
│   ├── validation.rs     # Input validation
│   └── supabase.rs       # Supabase client utilities
└── config/               # Configuration
    ├── mod.rs
    └── settings.rs
```

#### Cargo.toml Dependencies
```toml
[dependencies]
# Core async runtime
tokio = { version = "1.35", features = ["macros", "rt"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Core utilities
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }

# Web framework
axum = { version = "0.7", features = ["json"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# AWS Lambda
lambda_runtime = "0.8"
lambda_web = "0.2" 
aws_lambda_events = "0.11"

# Supabase integration
supabase-auth = "0.3"
postgrest = "1.5"
jsonwebtoken = "9.2"  # Still needed for Supabase JWT verification

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Validation
validator = { version = "0.16", features = ["derive"] }

# Additional utilities for Supabase
base64 = "0.21"
hmac = "0.12"
sha2 = "0.10"
```

### 8.2 Core Implementation Examples

#### Supabase Authentication Middleware
```rust
// middleware/supabase_auth.rs
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseClaims {
    pub aud: String,                    // Audience
    pub exp: usize,                     // Expiration
    pub sub: String,                    // User ID (from auth.users)
    pub email: Option<String>,          // User email
    pub phone: Option<String>,          // User phone
    pub app_metadata: serde_json::Value, // App metadata
    pub user_metadata: serde_json::Value, // User metadata
    pub role: String,                   // Supabase role (authenticated/anon)
    pub aal: Option<String>,            // Authentication Assurance Level
    pub amr: Option<Vec<serde_json::Value>>, // Authentication Method Reference
    pub session_id: Option<String>,     // Session ID
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub email: String,
    pub school_id: Uuid,
    pub role: String,           // Application role (parent/admin/teacher)
    pub supabase_role: String,  // Supabase role (authenticated/anon)
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub async fn supabase_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract JWT token from Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(token) => token,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // Verify Supabase JWT
    let auth_context = verify_supabase_jwt(token, &state.db)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add auth context to request extensions
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

async fn verify_supabase_jwt(
    token: &str, 
    db: &PgPool
) -> Result<AuthContext, Box<dyn std::error::Error + Send + Sync>> {
    // Get Supabase JWT secret from environment
    let jwt_secret = env::var("SUPABASE_JWT_SECRET")?;
    
    // Set up JWT validation
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]);
    
    // Decode and validate JWT
    let token_data = decode::<SupabaseClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )?;

    let claims = token_data.claims;
    let user_id = Uuid::parse_str(&claims.sub)?;

    // Get user profile from database to get school context and application role
    let profile = sqlx::query!(
        r#"
        SELECT 
            p.school_id,
            p.role,
            p.first_name,
            p.last_name,
            p.is_active,
            au.email
        FROM profiles p
        JOIN auth.users au ON p.id = au.id
        WHERE p.id = $1 AND p.is_active = true
        "#,
        user_id
    )
    .fetch_optional(db)
    .await?;

    let profile = profile.ok_or("User profile not found or inactive")?;

    // Ensure user belongs to a school
    if profile.school_id.is_none() {
        return Err("User has no school association".into());
    }

    Ok(AuthContext {
        user_id,
        email: profile.email.unwrap_or_default(),
        school_id: profile.school_id.unwrap(),
        role: profile.role,
        supabase_role: claims.role,
        session_id: claims.session_id,
    })
}

// Extractor for auth context
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts, 
        _state: &S
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

// Helper function to check if user has required role
impl AuthContext {
    pub fn has_role(&self, required_role: &str) -> bool {
        match required_role {
            "super_admin" => self.role == "super_admin",
            "admin" => matches!(self.role.as_str(), "admin" | "super_admin"),
            "teacher" => matches!(self.role.as_str(), "teacher" | "admin" | "super_admin"),
            "parent" => matches!(self.role.as_str(), "parent" | "teacher" | "admin" | "super_admin"),
            _ => false,
        }
    }

    pub fn is_same_school(&self, school_id: Uuid) -> bool {
        self.school_id == school_id || self.role == "super_admin"
    }
    
    pub fn can_access_user(&self, target_user_id: Uuid) -> bool {
        // Users can access their own profile
        if self.user_id == target_user_id {
            return true;
        }
        
        // Admins can access users in their school
        matches!(self.role.as_str(), "admin" | "super_admin")
    }
}
```

#### User Management Handler
```rust
// handlers/users.rs
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::middleware::auth::AuthContext;
use crate::services::invite_service::{InviteService, CreateInviteRequest};
use crate::models::user::{User, UserRole};
use crate::utils::error::{ApiError, ApiResult};

#[derive(Deserialize)]
pub struct ListUsersQuery {
    pub role: Option<UserRole>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
}

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateInvitePayload {
    #[validate(email)]
    pub email: String,
    pub role: UserRole,
    #[validate(length(min = 1, max = 100))]
    pub first_name: String,
    #[validate(length(min = 1, max = 100))]
    pub last_name: String,
    #[validate(length(min = 10, max = 20))]
    pub phone: Option<String>,
    pub send_welcome_email: Option<bool>,
}

pub async fn list_users(
    auth: AuthContext,
    Query(query): Query<ListUsersQuery>,
    State(user_service): State<UserService>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check permissions
    if !matches!(auth.role.as_str(), "admin" | "super_admin") {
        return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
    }

    let users = user_service
        .list_users(
            auth.school_id,
            query.role,
            query.page.unwrap_or(1),
            query.limit.unwrap_or(20),
        )
        .await?;

    Ok(Json(serde_json::json!({
        "data": users.data,
        "meta": {
            "page": users.page,
            "per_page": users.per_page,
            "total": users.total,
            "total_pages": users.total_pages
        }
    })))
}

pub async fn create_invite(
    auth: AuthContext,
    State(invite_service): State<InviteService>,
    Json(payload): Json<CreateInvitePayload>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate permissions
    if !matches!(auth.role.as_str(), "admin" | "super_admin") {
        return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
    }

    // Validate input
    payload.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    let request = CreateInviteRequest {
        email: payload.email,
        role: payload.role,
        first_name: payload.first_name,
        last_name: payload.last_name,
        phone: payload.phone,
        school_id: auth.school_id,
        send_welcome_email: payload.send_welcome_email.unwrap_or(true),
        created_by: auth.user_id,
    };

    // Send invitation via Supabase Admin API
    let invite = invite_service.send_invitation(request).await?;
    
    Ok(Json(serde_json::json!({
        "data": {
            "id": invite.id,
            "email": invite.email,
            "invite_url": invite.invite_url,
            "expires_at": invite.expires_at,
            "status": "pending"
        }
    })))
}

pub async fn get_user(
    auth: AuthContext,
    Path(user_id): Path<Uuid>,
    State(user_service): State<UserService>,
) -> ApiResult<Json<User>> {
    // Parents can only access their own profile
    if auth.role == "parent" && auth.user_id != user_id {
        return Err(ApiError::Forbidden("Can only access own profile".to_string()));
    }

    let user = user_service
        .get_user(auth.school_id, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(user))
}
```

#### Form Management Service
```rust
// services/form_service.rs
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::form::{FormTemplate, StudentFormAssignment, FormSubmission};
use crate::utils::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct FormService {
    db: PgPool,
}

impl FormService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn assign_forms_to_enrollment(
        &self,
        enrollment_id: Uuid,
        school_id: Uuid,
        child_id: Uuid,
        classroom_id: Uuid,
    ) -> ApiResult<i32> {
        let assignment_count: i32 = sqlx::query_scalar!(
            r#"
            SELECT assign_forms_to_enrollment($1, $2, $3, $4)
            "#,
            enrollment_id,
            school_id,
            child_id,
            classroom_id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?
        .unwrap_or(0);

        Ok(assignment_count)
    }

    pub async fn get_enrollment_forms(
        &self,
        school_id: Uuid,
        enrollment_id: Uuid,
    ) -> ApiResult<Vec<StudentFormAssignment>> {
        let assignments = sqlx::query_as!(
            StudentFormAssignment,
            r#"
            SELECT 
                sfa.id,
                sfa.school_id,
                sfa.enrollment_id,
                sfa.child_id,
                sfa.form_template_id,
                sfa.assignment_source,
                sfa.is_required,
                sfa.assigned_at,
                ft.form_name,
                ft.form_type,
                ft.fillout_form_url,
                ft.fillout_form_id,
                fs.id as submission_id,
                fs.submitted_at,
                CASE 
                    WHEN fs.id IS NOT NULL THEN 'completed'
                    ELSE 'pending'
                END as status
            FROM student_form_assignments sfa
            JOIN form_templates ft ON sfa.form_template_id = ft.id
            LEFT JOIN form_submissions fs ON sfa.id = fs.student_form_assignment_id
            WHERE sfa.school_id = $1 AND sfa.enrollment_id = $2
            ORDER BY ft.display_order ASC, ft.form_name ASC
            "#,
            school_id,
            enrollment_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(assignments)
    }

    pub async fn process_fillout_webhook(
        &self,
        webhook_data: serde_json::Value,
    ) -> ApiResult<()> {
        // Extract enrollment and assignment context from webhook data
        let enrollment_id = webhook_data["data"]["enrollment_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| ApiError::Validation("Invalid enrollment_id".to_string()))?;

        let assignment_id = webhook_data["data"]["assignment_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok());

        let fillout_submission_id = webhook_data["submission_id"]
            .as_str()
            .ok_or_else(|| ApiError::Validation("Missing submission_id".to_string()))?;

        // Store form submission
        let submission_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO form_submissions (
                id,
                school_id,
                enrollment_id,
                student_form_assignment_id,
                form_template_id,
                fillout_submission_id,
                form_data,
                metadata,
                submitted_at
            ) VALUES (
                $1,
                (SELECT school_id FROM enrollments WHERE id = $2),
                $2,
                $3,
                (SELECT form_template_id FROM student_form_assignments WHERE id = $3),
                $4,
                $5,
                $6,
                $7
            )
            "#,
            submission_id,
            enrollment_id,
            assignment_id,
            fillout_submission_id,
            webhook_data["data"],
            webhook_data["metadata"],
            webhook_data["created_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now)
        )
        .execute(&self.db)
        .await
        .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(())
    }
}
```

---

## 9. OpenAPI 3.0 Specification

### 9.1 OpenAPI Schema Definition

```yaml
openapi: 3.0.3
info:
  title: The Goddard School Enrollment Management System API
  version: 1.0.0
  description: |
    REST API for The Goddard School multi-tenant enrollment management system.
    
    ## Authentication
    All endpoints require Bearer JWT tokens with school context.
    
    ## Multi-Tenancy
    All resources are automatically scoped to the authenticated user's school.
    
    ## Rate Limits
    - 1000 requests/minute per school
    - 100 requests/minute per user
  contact:
    name: The Goddard School API Team
    url: https://api.goddard.com/support
    email: api-support@goddard.com
  license:
    name: MIT
    url: https://opensource.org/licenses/MIT

servers:
  - url: https://api.goddard.com/v1
    description: Production API
  - url: https://staging-api.goddard.com/v1
    description: Staging API

security:
  - BearerAuth: []

components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT

  parameters:
    SchoolId:
      name: school_id
      in: header
      required: false
      description: School context (automatically resolved from user profile via Supabase Auth)
      schema:
        type: string
        format: uuid
    
    Page:
      name: page
      in: query
      description: Page number for pagination
      schema:
        type: integer
        minimum: 1
        default: 1
    
    Limit:
      name: limit
      in: query
      description: Number of items per page
      schema:
        type: integer
        minimum: 1
        maximum: 100
        default: 20

  schemas:
    # Core Models
    School:
      type: object
      properties:
        id:
          type: string
          format: uuid
          description: Unique identifier for the school
        name:
          type: string
          description: School name
        subdomain:
          type: string
          description: School subdomain (e.g., 'brookside')
        settings:
          type: object
          description: School-specific configuration
        is_active:
          type: boolean
          default: true
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
      required:
        - id
        - name
        - subdomain

    User:
      type: object
      properties:
        id:
          type: string
          format: uuid
        school_id:
          type: string
          format: uuid
        email:
          type: string
          format: email
        role:
          type: string
          enum: [parent, teacher, admin, super_admin]
        first_name:
          type: string
        last_name:
          type: string
        phone:
          type: string
          nullable: true
        is_active:
          type: boolean
          default: true
        email_verified:
          type: boolean
          default: false
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
        last_login:
          type: string
          format: date-time
          nullable: true
      required:
        - id
        - school_id
        - email
        - role
        - first_name
        - last_name

    Child:
      type: object
      properties:
        id:
          type: string
          format: uuid
        parent_id:
          type: string
          format: uuid
        school_id:
          type: string
          format: uuid
        first_name:
          type: string
        last_name:
          type: string
        birth_date:
          type: string
          format: date
        age_group:
          type: string
          enum: [infant, toddler, preschool, pre_k]
        medical_info:
          type: object
          properties:
            allergies:
              type: array
              items:
                type: string
            medications:
              type: array
              items:
                type: string
            special_needs:
              type: string
              nullable: true
            emergency_medical_info:
              type: string
              nullable: true
        enrollment_status:
          type: string
          enum: [pending, enrolled, graduated, withdrawn]
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
      required:
        - id
        - parent_id
        - school_id
        - first_name
        - last_name
        - birth_date

    Enrollment:
      type: object
      properties:
        id:
          type: string
          format: uuid
        child_id:
          type: string
          format: uuid
        school_id:
          type: string
          format: uuid
        classroom_id:
          type: string
          format: uuid
        status:
          type: string
          enum: [pending, in_progress, under_review, approved, rejected]
        progress:
          type: object
          properties:
            total_forms:
              type: integer
            completed_forms:
              type: integer
            pending_forms:
              type: integer
            completion_percentage:
              type: number
              format: float
        start_date:
          type: string
          format: date
          nullable: true
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
      required:
        - id
        - child_id
        - school_id
        - status

    FormTemplate:
      type: object
      properties:
        id:
          type: string
          format: uuid
        school_id:
          type: string
          format: uuid
        form_name:
          type: string
        form_description:
          type: string
          nullable: true
        form_type:
          type: string
          enum: [admission, medical, emergency, authorization, handbook, agreement]
        fillout_form_id:
          type: string
          description: Fillout.com form identifier
        fillout_form_url:
          type: string
          format: uri
          description: Direct URL to Fillout form
        status:
          type: string
          enum: [active, school_default, draft, archive]
        is_required:
          type: boolean
          default: false
        display_order:
          type: integer
          default: 0
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
      required:
        - id
        - school_id
        - form_name
        - fillout_form_id
        - fillout_form_url
        - status

    # Error Models
    ApiError:
      type: object
      properties:
        type:
          type: string
          format: uri
          description: Error type identifier
        title:
          type: string
          description: Human-readable error title
        status:
          type: integer
          description: HTTP status code
        detail:
          type: string
          description: Detailed error description
        instance:
          type: string
          description: API endpoint that generated the error
        school_id:
          type: string
          format: uuid
          description: School context for the request
        request_id:
          type: string
          description: Unique request identifier for support
        timestamp:
          type: string
          format: date-time
        errors:
          type: array
          items:
            type: object
            properties:
              field:
                type: string
              code:
                type: string
              message:
                type: string
      required:
        - type
        - title
        - status
        - detail

    # Response Wrappers
    PaginatedResponse:
      type: object
      properties:
        data:
          type: array
        meta:
          type: object
          properties:
            page:
              type: integer
            per_page:
              type: integer
            total:
              type: integer
            total_pages:
              type: integer
        links:
          type: object
          properties:
            self:
              type: string
              format: uri
            next:
              type: string
              format: uri
              nullable: true
            prev:
              type: string
              format: uri
              nullable: true
            first:
              type: string
              format: uri
            last:
              type: string
              format: uri

  responses:
    BadRequest:
      description: Bad Request
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ApiError'
          example:
            type: "https://api.goddard.com/errors/bad-request"
            title: "Bad Request"
            status: 400
            detail: "The request could not be processed"
            instance: "/api/v1/children"

    Unauthorized:
      description: Authentication required
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ApiError'
          example:
            type: "https://api.goddard.com/errors/authentication-error"
            title: "Authentication Required"
            status: 401
            detail: "Valid authentication token is required"

    Forbidden:
      description: Insufficient permissions
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ApiError'
          example:
            type: "https://api.goddard.com/errors/authorization-error"
            title: "Insufficient Permissions"
            status: 403
            detail: "You do not have permission to access this resource"

    NotFound:
      description: Resource not found
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ApiError'
          example:
            type: "https://api.goddard.com/errors/not-found"
            title: "Resource Not Found"
            status: 404
            detail: "The requested resource could not be found"

    ValidationError:
      description: Validation failed
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ApiError'
          example:
            type: "https://api.goddard.com/errors/validation-error"
            title: "Validation Error"
            status: 422
            detail: "One or more fields failed validation"
            errors:
              - field: "email"
                code: "invalid_format"
                message: "Email must be a valid email address"

    RateLimitExceeded:
      description: Rate limit exceeded
      content:
        application/json:
          schema:
            allOf:
              - $ref: '#/components/schemas/ApiError'
              - type: object
                properties:
                  retry_after:
                    type: integer
                  limit:
                    type: integer
                  remaining:
                    type: integer
                  reset_time:
                    type: string
                    format: date-time

paths:
  # Note: Authentication is handled client-side by Supabase Auth
  # All endpoints below require Bearer JWT token from Supabase in Authorization header

  # School Endpoints
  /school:
    get:
      tags: [Schools]
      summary: Get current school details
      description: Returns details for the authenticated user's school
      responses:
        '200':
          description: School details
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    $ref: '#/components/schemas/School'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '403':
          $ref: '#/components/responses/Forbidden'

  # User Endpoints
  /users:
    get:
      tags: [Users]
      summary: List users
      description: Get paginated list of users in the school
      parameters:
        - $ref: '#/components/parameters/Page'
        - $ref: '#/components/parameters/Limit'
        - name: role
          in: query
          description: Filter by user role
          schema:
            type: string
            enum: [parent, teacher, admin, super_admin]
        - name: sort
          in: query
          description: Sort field
          schema:
            type: string
            enum: [first_name, last_name, email, created_at]
            default: last_name
      responses:
        '200':
          description: Users list
          content:
            application/json:
              schema:
                allOf:
                  - $ref: '#/components/schemas/PaginatedResponse'
                  - type: object
                    properties:
                      data:
                        type: array
                        items:
                          $ref: '#/components/schemas/User'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '403':
          $ref: '#/components/responses/Forbidden'

    post:
      tags: [Users]
      summary: Create new user
      description: Create a new user in the school (admin only)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                email:
                  type: string
                  format: email
                role:
                  type: string
                  enum: [parent, teacher, admin]
                first_name:
                  type: string
                  minLength: 1
                  maxLength: 100
                last_name:
                  type: string
                  minLength: 1
                  maxLength: 100
                phone:
                  type: string
                  nullable: true
                send_welcome_email:
                  type: boolean
                  default: true
              required:
                - email
                - role
                - first_name
                - last_name
      responses:
        '201':
          description: User created successfully
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    $ref: '#/components/schemas/User'
        '400':
          $ref: '#/components/responses/BadRequest'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '403':
          $ref: '#/components/responses/Forbidden'
        '422':
          $ref: '#/components/responses/ValidationError'

  /users/{user_id}:
    parameters:
      - name: user_id
        in: path
        required: true
        schema:
          type: string
          format: uuid
    get:
      tags: [Users]
      summary: Get user details
      description: Get details of a specific user
      responses:
        '200':
          description: User details
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    $ref: '#/components/schemas/User'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '403':
          $ref: '#/components/responses/Forbidden'
        '404':
          $ref: '#/components/responses/NotFound'

  # Children Endpoints
  /children:
    get:
      tags: [Children]
      summary: List children
      description: Get list of children (parents see own children, admins see all)
      parameters:
        - $ref: '#/components/parameters/Page'
        - $ref: '#/components/parameters/Limit'
        - name: parent_id
          in: query
          description: Filter by parent ID
          schema:
            type: string
            format: uuid
      responses:
        '200':
          description: Children list
          content:
            application/json:
              schema:
                allOf:
                  - $ref: '#/components/schemas/PaginatedResponse'
                  - type: object
                    properties:
                      data:
                        type: array
                        items:
                          $ref: '#/components/schemas/Child'

    post:
      tags: [Children]
      summary: Add new child
      description: Add a new child to the parent's account
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                first_name:
                  type: string
                last_name:
                  type: string
                birth_date:
                  type: string
                  format: date
                medical_info:
                  type: object
                  properties:
                    allergies:
                      type: array
                      items:
                        type: string
                    medications:
                      type: array
                      items:
                        type: string
              required:
                - first_name
                - last_name
                - birth_date
      responses:
        '201':
          description: Child added successfully
          content:
            application/json:
              schema:
                type: object
                properties:
                  data:
                    $ref: '#/components/schemas/Child'

tags:
  - name: Authentication
    description: Authentication and authorization endpoints
  - name: Schools
    description: School management
  - name: Users
    description: User management
  - name: Children
    description: Children management
  - name: Enrollments
    description: Enrollment management
  - name: Forms
    description: Form management
  - name: Documents
    description: Document management
  - name: Communications
    description: Email and notifications
  - name: Administration
    description: Administrative functions
```

### 9.2 Implementation Notes for OpenAPI

#### Code Generation
The OpenAPI specification can be used to generate:
- **Client SDKs**: TypeScript, JavaScript, Python, Java clients
- **Server Stubs**: Rust Axum server templates with proper routing
- **API Documentation**: Interactive Swagger UI documentation
- **Validation**: Request/response validation middleware

#### Rust Integration
```rust
// Integration with the utoipa crate for Rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::users::list_users,
        handlers::invites::create_invite,
        handlers::children::list_children,
    ),
    components(
        schemas(User, Child, Enrollment, FormTemplate, ApiError)
    ),
    tags(
        (name = "Authentication", description = "Auth endpoints"),
        (name = "Users", description = "User management")
    ),
    info(
        title = "Goddard School API",
        version = "1.0.0",
        description = "Multi-tenant school enrollment management"
    )
)]
struct ApiDoc;

// Generate OpenAPI spec at runtime
pub fn generate_openapi_spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
```

---

## Quick Reference

### Common Response Codes
- `200` - Success
- `201` - Created
- `204` - No Content (DELETE success)
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (duplicate resource)
- `422` - Validation Error
- `429` - Rate Limited
- `500` - Internal Server Error

### Pagination Query Parameters
- `page` - Page number (default: 1)
- `limit` - Items per page (default: 20, max: 100)
- `sort` - Sort field (default: created_at)
- `order` - Sort order: asc|desc (default: desc)

### Filtering Query Parameters
- `status` - Filter by status
- `role` - Filter by user role
- `type` - Filter by form type
- `created_after` - ISO date filter
- `created_before` - ISO date filter

### Multi-Tenant Context
All API endpoints automatically scope data to the authenticated user's school context. School context is resolved through:
1. JWT claims (`school_id`)
2. Origin header domain (`brookside.goddard.com`)
3. Custom header (`X-School-ID`)

### Rate Limits
- **Per School**: 1000 requests/minute
- **Per User**: 100 requests/minute
- **Webhook Endpoints**: 5000 requests/minute
- **File Uploads**: 10 requests/minute

### Authentication Headers
```http
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
Content-Type: application/json
X-School-ID: 550e8400-e29b-41d4-a716-446655440000 (optional)
```

### Standard Request/Response Patterns

#### List Resources
```http
GET /api/v1/{resource}?page=1&limit=20&sort=name&order=asc
```

#### Get Single Resource
```http
GET /api/v1/{resource}/{id}
```

#### Create Resource
```http
POST /api/v1/{resource}
Content-Type: application/json

{
  "field1": "value1",
  "field2": "value2"
}
```

#### Update Resource
```http
PATCH /api/v1/{resource}/{id}
Content-Type: application/json

{
  "field_to_update": "new_value"
}
```

#### Delete Resource
```http
DELETE /api/v1/{resource}/{id}
```

### Environment Variables (Rust Implementation)

```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost/goddard_db

# Authentication
JWT_SECRET=your-super-secret-jwt-key
JWT_EXPIRATION_HOURS=24

# Fillout Integration
FILLOUT_API_KEY=fillout_api_key
FILLOUT_WEBHOOK_SECRET=webhook_secret

# Email Service
RESEND_API_KEY=resend_api_key

# AWS Services
AWS_REGION=us-west-2
S3_BUCKET_NAME=goddard-documents

# Application
API_BASE_URL=https://api.goddard.com
CORS_ORIGINS=https://brookside.goddard.com,https://riverside.goddard.com

# Rate Limiting
RATE_LIMIT_PER_SCHOOL=1000
RATE_LIMIT_PER_USER=100
RATE_LIMIT_WINDOW_MINUTES=1
```

### Common Implementation Patterns

#### Error Handling in Rust
```rust
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "https://api.goddard.com/errors/not-found",
                msg,
            ),
            ApiError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "https://api.goddard.com/errors/validation-error",
                msg,
            ),
            // ... other error types
        };

        let error_response = json!({
            "type": error_type,
            "title": status.canonical_reason().unwrap_or("Unknown Error"),
            "status": status.as_u16(),
            "detail": message,
            "timestamp": chrono::Utc::now()
        });

        (status, Json(error_response)).into_response()
    }
}
```

#### Multi-Tenant Query Pattern
```rust
// Always include school_id in queries for RLS
async fn get_user_by_id(
    db: &PgPool,
    school_id: Uuid,
    user_id: Uuid,
) -> Result<Option<User>, ApiError> {
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE school_id = $1 AND id = $2",
        school_id,
        user_id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::Database(e.to_string()))?;

    Ok(user)
}
```

#### Form Assignment Business Logic
```rust
// Implement the 3-tier form assignment hierarchy
pub async fn calculate_student_forms(
    db: &PgPool,
    enrollment_id: Uuid,
    school_id: Uuid,
    classroom_id: Uuid,
) -> Result<Vec<FormAssignment>, ApiError> {
    // This uses the stored procedure defined in the database schema
    let assignments = sqlx::query_as!(
        FormAssignment,
        "SELECT * FROM assign_forms_to_enrollment($1, $2, $3, $4)",
        enrollment_id,
        school_id,
        // child_id will be extracted from enrollment
        classroom_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::Database(e.to_string()))?;

    Ok(assignments)
}
```

---

## Summary

This comprehensive API specification provides a complete foundation for implementing The Goddard School Enrollment Management System in Rust. The design emphasizes:

### ✅ **Key Strengths**

1. **Multi-Tenant Architecture**: Complete data isolation through RLS policies and Supabase Auth context
2. **Google Cloud API Compliance**: Follows resource-oriented design and RESTful principles  
3. **Scalable Design**: Supports 500+ concurrent users with optimized query patterns
4. **Security-First**: COPPA/FERPA compliant with comprehensive authentication and authorization
5. **Form Management Excellence**: Sophisticated 3-tier hierarchy with Fillout.com integration
6. **Production-Ready**: Complete error handling, rate limiting, and monitoring support
7. **Developer Experience**: OpenAPI specification, comprehensive examples, and clear patterns

### 🎯 **Business Value**

- **Digitizes Paper Process**: Transforms manual enrollment into streamlined digital experience
- **Multi-Location Support**: Each school operates independently with shared infrastructure
- **Enhanced Communication**: Multiple email management and automated notifications
- **Compliance Ready**: Built-in audit trails and data retention policies
- **Scalable Growth**: Architecture supports expansion from 10 to 100+ schools

### 🚀 **Implementation Readiness**

This specification provides everything needed for development:
- Complete endpoint definitions with examples
- Rust implementation patterns and code snippets
- Database schema with business logic functions
- Error handling and validation patterns
- Security and multi-tenancy implementation guides
- OpenAPI 3.0 specification for code generation

The API design successfully balances complexity with usability, providing powerful functionality while maintaining the simplicity that Google Cloud's API design principles emphasize. This foundation will enable The Goddard School to transform their enrollment process while maintaining the flexibility to grow and evolve their digital platform.

---

*This specification serves as the definitive guide for implementing a production-ready enrollment management system that meets the needs of parents, administrators, and regulatory requirements while providing a foundation for future enhancements.*
