local user_id = 1
local request_counter = 0
local ROTATE_EVERY = 10  -- Change user every 10 requests

local gateway_secret = os.getenv("APP_GATEWAY_SECRET") or "test-secret"
local alg = os.getenv("JWT_ALGORITHM") or "RS256"
local client_id = os.getenv("CLIENT_ID")

function request()
    -- Rotate token every ROTATE_EVERY requests
    request_counter = request_counter + 1
    if request_counter % ROTATE_EVERY == 0 then
        user_id = user_id + 1
    end

    local headers = {
        ["X-Gateway-Secret"] = gateway_secret,
        ["X-User-Id"] = user_id,
        ["X-Client-Id"] = client_id,
        ["X-Scope"] = "openid",
    }

    return wrk.format("GET", nil, headers)
end
