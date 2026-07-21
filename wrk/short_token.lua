local gateway_secret = os.getenv("GATEWAY_SECRET") or "test-secret"

-- validation error early. No crypto, no database
function request()
    local token = "short"

    local body = '{"token":"' .. token .. '"}'
    local headers = {
        ["X-Gateway-Secret"] = gateway_secret,
        ["Content-Type"] = "application/json"
    }

    return wrk.format("POST", nil, headers, body)
end
