import React from "react";
import { createIdentity } from "../../../../dist-js/index.js";
import { generateSeed } from "../library/seed.js";

export function CreateIdentity() {
    const onCreateIdentity = (name: string) => createIdentity(name, generateSeed(8));
    return <div>
        <button onClick={() => onCreateIdentity("test")} >Test</button>
    </div>;
}
