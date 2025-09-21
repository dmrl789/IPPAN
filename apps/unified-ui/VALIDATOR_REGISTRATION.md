# Validator Registration Feature

## Overview
The validator registration feature allows users to become validators on the IPPAN network by submitting a registration form through the web interface.

## Components

### ValidatorRegistrationModal
- **Location**: `src/components/ValidatorRegistrationModal.tsx`
- **Purpose**: Modal dialog for validator registration
- **Features**:
  - Form validation for all required fields
  - Real-time error feedback
  - Loading states during submission
  - Toast notifications for success/error

### Toast Notification System
- **Location**: `src/components/Toast.tsx`
- **Purpose**: User feedback system for notifications
- **Features**:
  - Success, error, warning, and info toast types
  - Auto-dismiss with customizable duration
  - Smooth animations
  - Manual dismiss option

## API Endpoints

### POST /api/v1/validators/register
Registers a new validator on the network.

**Request Body**:
```json
{
  "node_id": "string",
  "stake_amount": "number",
  "public_key": "string",
  "commission_rate": "number",
  "moniker": "string",
  "website": "string (optional)",
  "description": "string (optional)"
}
```

**Response**:
```json
{
  "tx_id": "string",
  "status": "pending"
}
```

### GET /api/v1/validators
Retrieves the list of all validators.

**Response**:
```json
[
  {
    "node_id": "string",
    "moniker": "string",
    "stake_amount": "number",
    "public_key": "string",
    "commission_rate": "number",
    "website": "string (optional)",
    "description": "string (optional)",
    "is_active": "boolean",
    "uptime_percentage": "number",
    "performance_score": "number",
    "total_blocks_produced": "number",
    "registration_time": "number"
  }
]
```

## Validation Rules

### Node ID
- Required field
- Must contain only letters, numbers, and underscores
- Example: `validator_node_001`

### Stake Amount
- Required field
- Minimum: 10,000 IPPAN
- Must be a valid number

### Public Key
- Required field
- Must be a valid hexadecimal string starting with `0x`
- Example: `0x1234567890abcdef1234567890abcdef12345678`

### Commission Rate
- Required field
- Must be between 0% and 100%
- Default: 5%

### Moniker (Validator Name)
- Required field
- Human-readable name for the validator
- Example: `My Validator`

### Website
- Optional field
- Must be a valid URL if provided
- Example: `https://myvalidator.com`

### Description
- Optional field
- Free text description of the validator

## Usage

1. Navigate to the Validators page
2. Click the "Become Validator" button
3. Fill out the registration form with required information
4. Submit the form
5. Receive confirmation via toast notification

## Error Handling

The system provides comprehensive error handling:

- **Form Validation**: Real-time validation with helpful error messages
- **API Errors**: Network and server errors are caught and displayed
- **Toast Notifications**: Success and error feedback via toast system
- **Loading States**: Visual feedback during form submission

## Future Enhancements

- Integration with actual blockchain consensus module
- Real-time validator status updates
- Advanced validator metrics and analytics
- Validator performance monitoring
- Slashing event notifications
