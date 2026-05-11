# Open Source

# Identity and Access Management

Add authentication to applications and secure services with minimum effort.

No need to deal with storing users or authenticating users.


[Get Started](https://www.keycloak.org/guides) [Download](https://www.keycloak.org/downloads)

![Keycloak](https://www.keycloak.org/resources/images/icon.svg)

[News](https://www.keycloak.org/blog)

07 May [Fine-Grained Admin Permissions for Organizations](https://www.keycloak.org/2026/05/org-fgap)

07 May [New Keycloak Maintainer: Ricardo Martin](https://www.keycloak.org/2026/05/new-maintainer-ricardo)

02 May [Announcing Keycloak's Identity Summit: KEYCONF26](https://www.keycloak.org/2026/05/keyconf26-prague-announce)

## Single-Sign On

Users authenticate with Keycloak rather than individual applications. This means that your applications
don't have to deal with login forms, authenticating users, and storing users. Once logged-in to
Keycloak, users don't have to login again to access a different application.


This also applies to logout. Keycloak provides single-sign out, which means users only have to logout once to be
logged-out of all applications that use Keycloak.


![Screenshot showing a user's login screen as presented by Keycloak](https://www.keycloak.org/resources/images/screen-login.png)

## Identity Brokering and Social Login

Enabling login with social networks is easy to add through the admin console. It's just a matter of selecting the
social network you want to add. No code or changes to your application is required.


Keycloak can also authenticate users with existing OpenID Connect or SAML 2.0 Identity Providers. Again, this is
just a matter of configuring the Identity Provider through the admin console.


![Diagram illustrating brokering](https://www.keycloak.org/resources/images/dia-identity-brokering.png)

## User Federation

Keycloak has built-in support to connect to existing LDAP or Active Directory servers. You can also implement your own
provider if you have users in other stores, such as a relational database.


![Diagram illustrating user federation](https://www.keycloak.org/resources/images/dia-user-fed.png)

## Admin Console

Through the admin console administrators can centrally manage all aspects of the Keycloak server.


They can enable and disable various features. They can configure identity brokering and user federation.


They can create and manage applications and services, and define fine-grained authorization
policies.


They can also manage users, including permissions and sessions.


![Screenshot of the admin console](https://www.keycloak.org/resources/images/screen-admin.png)

## Account Management Console

Through the account management console users can manage their own accounts. They can update the profile,
change passwords, and setup two-factor authentication.


Users can also manage sessions as well as view history for the account.


If you've enabled social login or identity brokering users can also link their accounts with additional
providers to allow them to authenticate to the same account with different identity providers.


![Screenshot of the account management console](https://www.keycloak.org/resources/images/screen-account.png)

## Standard Protocols

Keycloak is based on standard protocols and provides support for OpenID Connect, OAuth 2.0, and SAML.


![Logos of OpenID certification, SAML and OAuth 2.0](https://www.keycloak.org/resources/images/dia-protocols.png)

## Authorization Services

If role based authorization doesn't cover your needs, Keycloak provides fine-grained authorization services as well.
This allows you to manage permissions for all your services from the Keycloak admin console and gives you the
power to define exactly the policies you need.


Single-Sign OnLogin once to multiple applications

Standard ProtocolsOpenID Connect, OAuth 2.0 and SAML 2.0

Centralized ManagementFor admins and users

AdaptersSecure applications and services easily

LDAP and Active DirectoryConnect to existing user directories

Social LoginEasily enable social login

Identity BrokeringOpenID Connect or SAML 2.0 IdPs

High PerformanceLightweight, fast and scalable

ClusteringFor scalability and availability

ThemesCustomize look and feel

ExtensibleCustomize through code

Password PoliciesCustomize password policies