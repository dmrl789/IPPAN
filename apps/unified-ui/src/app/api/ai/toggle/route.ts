import { NextRequest, NextResponse } from 'next/server';

export async function POST(request: NextRequest) {
  try {
    const { enabled } = await request.json();
    
    // In a real implementation, this would toggle the actual AI service
    // For now, just return success
    console.log(`AI service ${enabled ? 'enabled' : 'disabled'}`);
    
    return NextResponse.json({ 
      success: true, 
      enabled,
      message: `AI service ${enabled ? 'enabled' : 'disabled'} successfully`
    });
  } catch (error) {
    console.error('Failed to toggle AI status:', error);
    return NextResponse.json(
      { error: 'Failed to toggle AI status' },
      { status: 500 }
    );
  }
}