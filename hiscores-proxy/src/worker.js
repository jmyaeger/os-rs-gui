/**
 * Cloudflare Worker proxy for OSRS Hiscores API
 *
 * This proxy allows browser-based apps to fetch hiscores data
 * without CORS restrictions.
 *
 * Usage: GET /?player=<username>
 * Returns: Raw hiscores data or error response
 */

const HISCORES_BASE_URL = 'https://secure.runescape.com/m=hiscore_oldschool/index_lite.ws';
const ALLOWED_ORIGINS = [
  'http://localhost:8080',
  'http://127.0.0.1:8080',
  'https://runesim.dev',
  'https://www.runesim.dev',
];

function getCorsHeaders(request) {
  const origin = request.headers.get('Origin');

  // Allow the origin if it's in our list, otherwise use the first allowed origin
  const allowedOrigin = ALLOWED_ORIGINS.includes(origin) ? origin : ALLOWED_ORIGINS[0];

  return {
    'Access-Control-Allow-Origin': allowedOrigin,
    'Access-Control-Allow-Methods': 'GET, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
    'Access-Control-Max-Age': '86400',
  };
}

async function handleRequest(request) {
  const url = new URL(request.url);
  const player = url.searchParams.get('player');
  const corsHeaders = getCorsHeaders(request);

  // Handle CORS preflight
  if (request.method === 'OPTIONS') {
    return new Response(null, { status: 204, headers: corsHeaders });
  }

  // Validate player parameter
  if (!player) {
    return new Response(
      JSON.stringify({ error: 'Missing required parameter: player' }),
      {
        status: 400,
        headers: {
          'Content-Type': 'application/json',
          ...corsHeaders,
        },
      }
    );
  }

  // Validate player name (alphanumeric, spaces, hyphens, underscores, 1-12 chars)
  const playerNameRegex = /^[\w\- ]{1,12}$/;
  if (!playerNameRegex.test(player)) {
    return new Response(
      JSON.stringify({ error: 'Invalid player name format' }),
      {
        status: 400,
        headers: {
          'Content-Type': 'application/json',
          ...corsHeaders,
        },
      }
    );
  }

  try {
    // Fetch from OSRS Hiscores API
    const hiscoresUrl = `${HISCORES_BASE_URL}?player=${encodeURIComponent(player)}`;
    const response = await fetch(hiscoresUrl, {
      headers: {
        'User-Agent': 'RuneSim DPS Calculator (runesim.dev)',
      },
    });

    // Handle not found
    if (response.status === 404) {
      return new Response(
        JSON.stringify({ error: `Player not found: ${player}` }),
        {
          status: 404,
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders,
          },
        }
      );
    }

    // Handle other errors
    if (!response.ok) {
      return new Response(
        JSON.stringify({ error: `Hiscores API error: ${response.status}` }),
        {
          status: response.status,
          headers: {
            'Content-Type': 'application/json',
            ...corsHeaders,
          },
        }
      );
    }

    // Return successful response
    const data = await response.text();
    return new Response(data, {
      status: 200,
      headers: {
        'Content-Type': 'text/plain',
        'Cache-Control': 'public, max-age=300', // Cache for 5 minutes
        ...corsHeaders,
      },
    });
  } catch (error) {
    return new Response(
      JSON.stringify({ error: 'Internal server error', details: error.message }),
      {
        status: 500,
        headers: {
          'Content-Type': 'application/json',
          ...corsHeaders,
        },
      }
    );
  }
}

export default {
  fetch: handleRequest,
};
