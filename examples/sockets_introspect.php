<?php

function introspect_token($token) {
    $socket_path = '/tmp/introspector.sock';

    $socket = stream_socket_client("unix://$socket_path", $errno, $errstr, 1.0);
    if (!$socket) {
        throw new RuntimeException("Failed to connect: $errstr ($errno)");
    }

    $params = [
        'token' => $token,
    ];
    var_dump($params);
    $request = json_encode($params);

    fwrite($socket, $request);
    fflush($socket);

    $response = fread($socket, 4096);
    fclose($socket);

    return json_decode($response, true);
}

function get_real_random_token() {
    $file = new SplFileObject('wrk/tokens_RS256.txt');
    $file->seek(rand(0, 999));

    return trim($file->current());
}

$result = introspect_token(get_real_random_token());

var_dump($result);
