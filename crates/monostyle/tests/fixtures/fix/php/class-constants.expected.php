<?php

class Routes
{
    public const HOME = '/home {kept}';
    public const API = '/api/v{n}';

    public function label(): string
    {
        return self::HOME . self::API;

    }
}
