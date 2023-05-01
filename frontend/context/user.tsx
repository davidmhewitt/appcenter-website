import { useRouter } from 'next/router';
import React, { ReactNode, createContext, useContext, useEffect, useState } from 'react'

type authContextType = {
    user: boolean;
    logout: () => void;
};

const authContextDefaultValues: authContextType = {
    user: false,
    logout: () => {},
};

const AuthContext = createContext<authContextType>(authContextDefaultValues);

export function useAuth() {
    return useContext(AuthContext);
}


type Props = {
    children: ReactNode;
}

export default function Provider({ children }: Props) {
    const router = useRouter();
    const [user, setUser] = useState(false);

    useEffect(() => {
        const getUserProfile = async () => {
            let res = await fetch("/api/users/test_auth")
            setUser(res.status == 200)
        }

        getUserProfile()
    })

    const logout = async () => {
        await fetch("/api/users/logout", {method: 'POST'})
        setUser(false);
        router.push("/");
    };

    const exposed = {
        user,
        logout,
    };

    return <AuthContext.Provider value={exposed}>{children}</AuthContext.Provider>;
}