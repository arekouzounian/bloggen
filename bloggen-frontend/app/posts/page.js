import fs from 'fs';
import path from 'path';

export default function Page() {
    const p = path.resolve('.', 'app', 'static'); 
    const posts = fs.readdirSync(p);

    
    return (
        <div className="text-center">
            <ul className="list-none bold underline">
                {posts.map((post, i) => <li key={i}><a href={"./posts/" + post}>{post}</a></li>)}
            </ul>
        </div>
    )
}